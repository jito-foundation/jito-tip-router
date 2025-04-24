use anyhow::{anyhow, Context, Result};
use clap::Parser;
use hostname::get as get_hostname_raw;
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use tokio::fs::read_dir;
use tokio::time::sleep;

/// A tool to continuously monitor and upload epoch-related files to Google Cloud Storage
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Directory to monitor for new files
    #[arg(short, long)]
    directory: String,

    /// Solana cluster (defaults to mainnet if not specified)
    #[arg(short, long, default_value = "mainnet")]
    cluster: String,

    /// Bucket name without gs:// prefix (defaults to jito-{cluster})
    #[arg(short, long)]
    bucket: Option<String>,

    /// Polling interval in seconds (defaults to 600 seconds / 10 minutes)
    #[arg(short, long, default_value = "600")]
    interval: u64,

    /// Directory to scan for snapshot files
    #[arg(short, long)]
    snapshot_directory: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments using Clap
    let args = Args::parse();

    // Verify directory exists
    let dir_path = Path::new(&args.directory);
    if !dir_path.exists() || !dir_path.is_dir() {
        return Err(anyhow!(
            "Directory not found or not a directory: {}",
            args.directory
        ));
    }

    // Verify snapshot directory exists
    let snapshot_dir_path = Path::new(&args.snapshot_directory);
    if !snapshot_dir_path.exists() || !snapshot_dir_path.is_dir() {
        return Err(anyhow!(
            "Snapshot directory not found or not a directory: {}",
            args.snapshot_directory
        ));
    }

    // Get hostname
    let hostname = get_hostname()?;

    // Determine bucket name
    let bucket_name = args
        .bucket
        .unwrap_or_else(|| format!("jito-{}", args.cluster));

    // Track already uploaded files
    let mut uploaded_files = HashSet::new();

    // Compile regex patterns for epoch files
    let merkle_pattern = Regex::new(r"^(\d+)_merkle_tree_collection\.json$").unwrap();
    let stake_pattern = Regex::new(r"^(\d+)_stake_meta_collection\.json$").unwrap();
    let snapshot_tar_zst_pattern = Regex::new(r"^snapshot-(\d+).*\.tar\.zst$").unwrap();

    let incremental_file_patterns = vec![&merkle_pattern, &stake_pattern];

    println!(
        "Starting file monitor in {} with {} second polling interval",
        args.directory, args.interval
    );
    println!("Looking for files matching patterns: '*_merkle_tree_collection.json' and '*_stake_meta_collection.json', and 'snapshot-*.tar.zst'");

    // Main monitoring loop
    loop {
        match scan_and_upload_files(
            dir_path,
            &bucket_name,
            &hostname,
            &mut uploaded_files,
            &incremental_file_patterns,
        )
        .await
        {
            Ok(uploaded) => {
                if uploaded > 0 {
                    println!("Uploaded {} new files", uploaded);
                }
            }
            Err(e) => {
                eprintln!("Error during scan/upload: {}", e);
            }
        }

        match scan_and_upload_snapshot_files(
            snapshot_dir_path,
            &bucket_name,
            &hostname,
            &mut uploaded_files,
            &[&snapshot_tar_zst_pattern],
        )
        .await
        {
            Ok(uploaded) => {
                if uploaded > 0 {
                    println!("Uploaded {} new snapshot files", uploaded);
                }
            }
            Err(e) => {
                eprintln!("Error during scan/upload: {}", e);
            }
        }

        // Wait for the next polling interval
        sleep(Duration::from_secs(args.interval)).await;
    }
}

/// Scans directory for matching files and uploads new ones
#[allow(clippy::arithmetic_side_effects)]
async fn scan_and_upload_files(
    dir_path: &Path,
    bucket_name: &str,
    hostname: &str,
    uploaded_files: &mut HashSet<String>,
    matching_patterns: &[&Regex],
) -> Result<usize> {
    let mut uploaded_count = 0;

    let mut entries = read_dir(dir_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Get filename as string
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip if already uploaded
        if uploaded_files.contains(&filename) {
            continue;
        }

        // Check if file matches our patterns
        let try_find_match: Option<&Regex> = matching_patterns
            .iter()
            .find(|re| re.captures(&filename).is_some())
            .copied();
        let try_epoch: Option<String> = try_find_match.and_then(|re| {
            re.captures(&filename)
                .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        });

        if let Some(epoch) = try_epoch {
            // We found a matching file, upload it
            if let Err(e) = upload_file(&path, &filename, &epoch, bucket_name, hostname).await {
                eprintln!("Failed to upload {}: {}", filename, e);
                continue;
            }

            // Mark as uploaded
            uploaded_files.insert(filename.clone());
            uploaded_count += 1;
        }
    }

    Ok(uploaded_count)
}

/// Scans directory for snapshots & uploads after deriving the associated epoch
#[allow(clippy::arithmetic_side_effects)]
async fn scan_and_upload_snapshot_files(
    dir_path: &Path,
    bucket_name: &str,
    hostname: &str,
    uploaded_files: &mut HashSet<String>,
    matching_patterns: &[&Regex],
) -> Result<usize> {
    let mut uploaded_count = 0;

    let mut entries = read_dir(dir_path).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Get filename as string
        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        // Skip if already uploaded
        if uploaded_files.contains(&filename) {
            continue;
        }

        // Check if file matches our patterns
        let try_find_match: Option<&Regex> = matching_patterns
            .iter()
            .find(|re| re.captures(&filename).is_some())
            .copied();

        let try_slot_num: Option<String> = try_find_match.and_then(|re| {
            re.captures(&filename)
                .and_then(|captures| captures.get(1).map(|m| m.as_str().to_string()))
        });

        if let Some(slot_num) = try_slot_num {
            let epoch = slot_num
                .parse::<u64>()
                .map_err(|_| {
                    anyhow::anyhow!("Failed to parse slot number from filename: {}", filename)
                })?
                .checked_div(432_000)
                .ok_or_else(|| {
                    anyhow::anyhow!("Failed to divide slot number by 432_000: {}", slot_num)
                })?
                .to_string();
            // We found a matching file, upload it
            if let Err(e) = upload_file(&path, &filename, &epoch, bucket_name, hostname).await {
                eprintln!("Failed to upload {}: {}", filename, e);
                continue;
            }
            // Mark as uploaded
            uploaded_files.insert(filename.clone());
            uploaded_count += 1;
        }
    }

    Ok(uploaded_count)
}

/// Uploads a single file to GCS using gcloud CLI
async fn upload_file(
    file_path: &Path,
    filename: &str,
    epoch: &str,
    bucket_name: &str,
    hostname: &str,
) -> Result<()> {
    // Create GCS object path (without bucket name)
    let filename = filename.replace("_", "-");
    let object_name = format!("{}/{}/{}", epoch, hostname, filename);
    println!("Uploading file: {}", file_path.display());
    println!("To GCS bucket: {}, object: {}", bucket_name, object_name);

    // Check if object already exists
    let check_output = Command::new("/opt/gcloud/google-cloud-sdk/bin/gcloud")
        .args([
            "storage",
            "objects",
            "describe",
            &format!("gs://{}/{}", bucket_name, object_name),
            "--format=json",
        ])
        .output()
        .with_context(|| "Failed to execute gcloud command to check if object exists")?;

    // If exit code is 0, file exists
    if check_output.status.success() {
        println!("File already exists in GCS. Skipping upload.");
        return Ok(());
    }

    // Upload to GCS
    let upload_status = Command::new("/opt/gcloud/google-cloud-sdk/bin/gcloud")
        .args([
            "storage",
            "cp",
            file_path
                .to_str()
                .ok_or_else(|| anyhow!("Invalid Unicode in file path: {}", file_path.display()))?,
            &format!("gs://{}/{}", bucket_name, object_name),
            "--content-type=application/json",
        ])
        .status()
        .with_context(|| format!("Failed to upload file to GCS: {}", file_path.display()))?;

    if !upload_status.success() {
        return Err(anyhow::anyhow!(
            "Failed to upload file: {}",
            file_path.display()
        ));
    }

    println!("Upload successful for {}", filename);
    Ok(())
}

fn get_hostname() -> Result<String> {
    let hostname = get_hostname_raw()
        .context("Failed to get hostname")?
        .to_string_lossy()
        .to_string();

    Ok(hostname)
}

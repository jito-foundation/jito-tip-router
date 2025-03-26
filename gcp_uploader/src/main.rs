use anyhow::{anyhow, Context, Result};
use clap::Parser;
use hostname::get as get_hostname_raw;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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

    /// Path to service account key JSON file for GCP authentication
    #[arg(short, long, env = "GOOGLE_APPLICATION_CREDENTIALS")]
    service_account_key: Option<String>,

    /// Polling interval in seconds (defaults to 600 seconds / 10 minutes)
    #[arg(short, long, default_value = "600")]
    interval: u64,
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

    println!(
        "Starting file monitor in {} with {} second polling interval",
        args.directory, args.interval
    );
    println!("Looking for files matching patterns: '*_merkle_tree_collection.json' and '*_stake_meta_collection.json'");

    // Main monitoring loop
    loop {
        match scan_and_upload_files(
            &dir_path,
            &bucket_name,
            &hostname,
            &mut uploaded_files,
            &merkle_pattern,
            &stake_pattern,
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

        // Wait for the next polling interval
        sleep(Duration::from_secs(args.interval)).await;
    }
}

/// Scans directory for matching files and uploads new ones
async fn scan_and_upload_files(
    dir_path: &Path,
    bucket_name: &str,
    hostname: &str,
    uploaded_files: &mut HashSet<String>,
    merkle_pattern: &Regex,
    stake_pattern: &Regex,
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
        let epoch = if let Some(captures) = merkle_pattern.captures(&filename) {
            captures.get(1).map(|m| m.as_str().to_string())
        } else if let Some(captures) = stake_pattern.captures(&filename) {
            captures.get(1).map(|m| m.as_str().to_string())
        } else {
            None
        };

        if let Some(epoch) = epoch {
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
    file_path: &PathBuf,
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
            file_path.to_str().unwrap(),
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

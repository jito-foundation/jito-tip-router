use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Env;
use hostname::get as get_hostname_raw;
use log::{error, info};
use regex::Regex;
use solana_client::rpc_client::RpcClient;
use solana_metrics::{datapoint_info, set_host_id};
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

    /// Solana JSON RPC URL to fetch current epoch for metrics
    #[arg(
        long,
        env = "RPC_URL",
        default_value = "https://api.mainnet-beta.solana.com"
    )]
    rpc_url: String,

    /// Path to gcloud executable
    #[arg(long, env = "GCLOUD_PATH", default_value = "/usr/bin/gcloud")]
    gcloud_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments using Clap
    let args = Args::parse();

    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

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

    set_host_id(hostname.clone());

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

    info!(
        "Starting file monitor in {} with {} second polling interval",
        args.directory, args.interval
    );
    info!("Looking for files matching patterns: '*_merkle_tree_collection.json' and '*_stake_meta_collection.json', and 'snapshot-*.tar.zst'");

    // Main monitoring loop
    loop {
        match scan_and_upload_files(
            dir_path,
            &bucket_name,
            &hostname,
            &mut uploaded_files,
            &incremental_file_patterns,
            &args.gcloud_path,
        )
        .await
        {
            Ok(uploaded) => {
                if uploaded > 0 {
                    info!("Uploaded {} new files", uploaded);
                }
            }
            Err(e) => {
                error!("Error during scan/upload: {}", e);
            }
        }

        match scan_and_upload_snapshot_files(
            snapshot_dir_path,
            &bucket_name,
            &hostname,
            &mut uploaded_files,
            &[&snapshot_tar_zst_pattern],
            &args.gcloud_path,
        )
        .await
        {
            Ok(uploaded) => {
                if uploaded > 0 {
                    info!("Uploaded {} new snapshot files", uploaded);
                }
            }
            Err(e) => {
                error!("Error during scan/upload: {}", e);
            }
        }

        // Emit metric about whether snapshot for current epoch is present in GCS
        if let Err(e) = emit_current_epoch_snapshot_metric(
            &args.rpc_url,
            &bucket_name,
            &hostname,
            &args.cluster,
            &args.gcloud_path,
        )
        .await
        {
            error!("Error emitting snapshot metric: {}", e);
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
    gcloud_path: &str,
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
            if let Err(e) = upload_file(&path, &filename, &epoch, bucket_name, hostname, gcloud_path).await {
                error!("Failed to upload {}: {}", filename, e);
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
#[allow(clippy::arithmetic_side_effects, clippy::integer_division)]
async fn scan_and_upload_snapshot_files(
    dir_path: &Path,
    bucket_name: &str,
    hostname: &str,
    uploaded_files: &mut HashSet<String>,
    matching_patterns: &[&Regex],
    gcloud_path: &str,
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
            if let Err(e) = upload_file(&path, &filename, &epoch, bucket_name, hostname, gcloud_path).await {
                error!("Failed to upload {}: {}", filename, e);
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
    gcloud_path: &str,
) -> Result<()> {
    // Create GCS object path (without bucket name)
    let filename = filename.replace("_", "-");
    let object_name = format!("{}/{}/{}", epoch, hostname, filename);
    info!("Uploading file: {}", file_path.display());
    info!("To GCS bucket: {}, object: {}", bucket_name, object_name);

    // Check if object already exists
    let check_output = Command::new(gcloud_path)
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
        info!("File already exists in GCS. Skipping upload.");
        return Ok(());
    }

    // Upload to GCS
    let upload_status = Command::new(gcloud_path)
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

    info!("Upload successful for {}", filename);
    Ok(())
}

async fn emit_current_epoch_snapshot_metric(
    rpc_url: &str,
    bucket_name: &str,
    hostname: &str,
    cluster: &str,
    gcloud_path: &str,
) -> Result<()> {
    // Fetch current epoch via Solana RPC
    let client = RpcClient::new(rpc_url.to_string());
    let epoch_info = client
        .get_epoch_info()
        .with_context(|| format!("Failed to fetch epoch info from {}", rpc_url))?;
    let epoch = epoch_info.epoch;
    let slot_index = epoch_info.slot_index;

    // Build GCS prefix path used by upload_file: {epoch}/{hostname}/snapshot-*.tar.zst
    // First, list objects under epoch/hostname and look for any snapshot-*.tar.zst
    let list_output = Command::new(gcloud_path)
        .args([
            "storage",
            "ls",
            &format!("gs://{}/{}/{}/", bucket_name, epoch, hostname),
        ])
        .output()
        .with_context(|| "Failed to execute gcloud ls for snapshot metric")?;

    let uploaded = if list_output.status.success() {
        let stdout = String::from_utf8_lossy(&list_output.stdout);
        stdout
            .lines()
            .any(|line| line.contains("snapshot-") && line.ends_with(".tar.zst"))
    } else {
        false
    };

    datapoint_info!(
        "tip_router_gcp_uploader.snapshot_present",
        ("epoch", epoch as i64, i64),
        ("slot_index", slot_index as i64, i64),
        ("present_i", uploaded as i64, i64),
        "cluster" => cluster,
        "hostname" => hostname,
        "bucket" => bucket_name,
    );

    Ok(())
}

fn get_hostname() -> Result<String> {
    let hostname = get_hostname_raw()
        .context("Failed to get hostname")?
        .to_string_lossy()
        .to_string();

    Ok(hostname)
}

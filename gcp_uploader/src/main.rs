use anyhow::{anyhow, Context, Result};
use clap::Parser;
use cloud_storage::{Client, ListRequest};
use futures_util::StreamExt;
use hostname::get as get_hostname_raw;
use regex::Regex;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs::{read_dir, File};
use tokio::io::AsyncReadExt;
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

    // Create GCS client with explicit authentication
    let client = create_gcs_client(&args.service_account_key);

    // Track already uploaded files
    let mut uploaded_files = HashSet::new();

    // Compile regex patterns for epoch files
    let merkle_pattern = Regex::new(r"^(\d+)_generated_merkle_tree\.json$").unwrap();
    let stake_pattern = Regex::new(r"^(\d+)_stake_meta\.json$").unwrap();

    println!(
        "Starting file monitor in {} with {} second polling interval",
        args.directory, args.interval
    );
    println!("Looking for files matching patterns: '*_generated_merkle_tree.json' and '*_stake_meta.json'");

    // Main monitoring loop
    loop {
        match scan_and_upload_files(
            &dir_path,
            &client,
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
    client: &Client,
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
            if let Err(e) =
                upload_file(&path, &filename, &epoch, client, bucket_name, hostname).await
            {
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

/// Uploads a single file to GCS
async fn upload_file(
    file_path: &PathBuf,
    filename: &str,
    epoch: &str,
    client: &Client,
    bucket_name: &str,
    hostname: &str,
) -> Result<()> {
    // Create GCS object path (without bucket name)
    let object_name = format!("{}/{}/{}", epoch, hostname, filename);

    println!("Uploading file: {}", file_path.display());
    println!("To GCS bucket: {}, object: {}", bucket_name, object_name);

    // Check if object already exists by using list with prefix
    let mut list_request = ListRequest::default();
    list_request.prefix = Some(object_name.clone());
    list_request.max_results = Some(1); // We only need to check existence

    let objects_stream = client.object().list(bucket_name, list_request).await?;

    // Pin the stream to the stack to handle the Unpin requirement
    let mut pinned_stream = Box::pin(objects_stream);

    // Process the stream items
    let exists = if let Some(result) = pinned_stream.next().await {
        match result {
            Ok(object_list) => object_list
                .items
                .iter()
                .any(|object| object.name == object_name),
            Err(_) => false,
        }
    } else {
        false
    };

    if exists {
        println!("File already exists in GCS. Skipping upload.");
        return Ok(());
    }

    // Read file content
    let mut file = File::open(file_path)
        .await
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;

    let mut content = Vec::new();
    file.read_to_end(&mut content)
        .await
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    // Upload to GCS
    client
        .object()
        .create(
            bucket_name,
            content,
            &object_name,
            "application/json", // JSON mime type
        )
        .await?;

    println!("Upload successful for {}", filename);

    Ok(())
}

/// Creates a GCS client with explicit authentication
fn create_gcs_client(service_account_path: &Option<String>) -> Client {
    // If a service account key path is provided, set the environment variable
    if let Some(key_path) = service_account_path {
        std::env::set_var("GOOGLE_APPLICATION_CREDENTIALS", key_path);
        println!("Using service account key from: {}", key_path);
    } else {
        println!("No service account key provided. Using default credentials.");
    }

    // Create client using the environment variable
    Client::new()
}

fn get_hostname() -> Result<String> {
    let hostname = get_hostname_raw()
        .context("Failed to get hostname")?
        .to_string_lossy()
        .to_string();

    Ok(hostname)
}

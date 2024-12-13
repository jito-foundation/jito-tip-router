use anyhow::Result;
use log::info;
use std::process::Command;
use std::time::Instant;
use solana_sdk::signer::keypair::Keypair;

pub struct SnapshotCreator {
    rpc_url: String,
    output_dir: String,
    max_snapshots: u32,
    compression: String,
    keypair: Keypair,
    ledger_path: std::path::PathBuf,
}

pub trait SnapshotCreatorTrait {
    async fn create_snapshot(&self, slot: u64) -> Result<()>;
}

// Implement the trait for the real SnapshotCreator
impl SnapshotCreatorTrait for SnapshotCreator {
    async fn create_snapshot(&self, slot: u64) -> Result<()> {
        // Call the internal implementation directly
        self.internal_create_snapshot(slot).await
    }
}

impl SnapshotCreator {
    pub fn new(
        rpc_url: &str,
        output_dir: String,
        max_snapshots: u32,
        compression: String,
        keypair: Keypair,
        ledger_path: std::path::PathBuf,
    ) -> Result<Self> {
        Ok(Self {
            rpc_url: rpc_url.to_string(),
            output_dir,
            max_snapshots,
            compression,
            keypair,
            ledger_path,
        })
    }

    pub async fn internal_create_snapshot(&self, slot: u64) -> Result<()> {
        let start_time = Instant::now();
        info!("Creating snapshot for slot {} using solana-ledger-tool", slot);
    
        // Ensure the ledger directory exists
        if !self.ledger_path.exists() {
            anyhow::bail!("Ledger directory does not exist: {:?}", self.ledger_path);
        }
    
        let status = Command::new("solana-ledger-tool")
            .args([
                "create-snapshot",
                &slot.to_string(),
                "--ledger",
                self.ledger_path.to_str().unwrap(),
                "--snapshot-archive-format",
                "zstd",
                "--maximum-full-snapshots-to-retain",
                &self.max_snapshots.to_string(),
                "--maximum-incremental-snapshots-to-retain",
                &self.max_snapshots.to_string(),
                "--output",
                "json",
                "--force-update-to-open",  // Added this flag
            ])
            .status()?;
    
        if !status.success() {
            // Get the error output for better debugging
            let output = Command::new("solana-ledger-tool")
                .args([
                    "create-snapshot",
                    &slot.to_string(),
                    "--ledger",
                    self.ledger_path.to_str().unwrap(),
                    "--snapshot-archive-format",
                    "zstd",
                    "--maximum-full-snapshots-to-retain",
                    &self.max_snapshots.to_string(),
                    "--maximum-incremental-snapshots-to-retain",
                    &self.max_snapshots.to_string(),
                    "--output",
                    "json",
                    "--force-update-to-open",
                ])
                .output()?;
    
            anyhow::bail!(
                "Failed to create snapshot using solana-ledger-tool: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    
        info!(
            "Snapshot creation completed in {:?}",
            start_time.elapsed()
        );
        Ok(())
    }
}
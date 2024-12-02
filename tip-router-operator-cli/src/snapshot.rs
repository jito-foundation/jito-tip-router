use anyhow::{ anyhow, Result };
use log::{ info, warn };
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    clock::Slot,
    epoch_schedule::EpochSchedule,
    signature::Keypair,
    genesis_config::GenesisConfig,
};
use solana_metrics::{ datapoint_info, datapoint_error };
use solana_ledger::{ bank_forks_utils, blockstore::Blockstore };
use std::{ path::PathBuf, fs, sync::Arc };
use std::sync::atomic::AtomicBool;
use solana_ledger::blockstore_processor::ProcessOptions;
use solana_runtime::{ self, snapshot_utils::SnapshotVersion, snapshot_bank_utils };
use std::num::NonZeroUsize;
use solana_runtime::snapshot_utils::ArchiveFormat;
use ellipsis_client::EllipsisClient;
use std::collections::HashMap;
use serde::{ Serialize, Deserialize };
use solana_stake_program;
use std::time::Instant;

#[derive(Serialize, Deserialize, Debug)]
struct StakeMetaAccount {
    lamports: u64,
    owner: String,
    stake_authority: Option<String>,
    withdraw_authority: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ValidatorStakeMeta {
    vote_account: String,
    identity: String,
    commission: u8,
    stake_accounts: HashMap<String, StakeMetaAccount>,
    total_stake: u64,
}

#[derive(Debug)]
pub struct SnapshotProgress {
    pub slot: Slot,
    pub phase: String,
    pub progress: f32,
    pub message: String,
}
pub struct SnapshotCreator {
    rpc_client: EllipsisClient,
    output_dir: PathBuf,
    max_snapshots: u32,
    compression: String,
    keypair: Arc<Keypair>,
    blockstore_path: PathBuf,
}

impl SnapshotCreator {
    pub fn new(
        rpc_url: &str,
        output_dir: String,
        max_snapshots: u32,
        compression: String,
        keypair: Keypair,
        blockstore_path: PathBuf
    ) -> Result<Self> {
        let rpc_client = EllipsisClient::from_rpc(RpcClient::new(rpc_url), &keypair)?;
        Ok(Self {
            rpc_client,
            output_dir: PathBuf::from(output_dir),
            max_snapshots,
            compression,
            keypair: Arc::new(keypair),
            blockstore_path,
        })
    }

    pub fn report_progress(&self, progress: SnapshotProgress) {
        info!(
            "Snapshot progress - Slot: {}, Phase: {}, Progress: {:.2}%, Message: {}",
            progress.slot,
            progress.phase,
            progress.progress * 100.0,
            progress.message
        );

        datapoint_info!(
            "tip_router_snapshot_progress",
            ("slot", progress.slot, i64),
            ("phase", progress.phase, String),
            ("progress", progress.progress, f64),
            ("message", progress.message, String)
        );
    }

    pub async fn monitor_epoch_boundary(&self) -> Result<()> {
        let epoch_schedule = self.rpc_client.get_epoch_schedule()?;
        let mut last_epoch = self.rpc_client.get_epoch_info()?.epoch;

        info!("Starting epoch boundary monitoring");
        datapoint_info!("tip_router_monitor", ("event", "start", String));

        loop {
            let slot = self.rpc_client.get_slot()?;
            let current_epoch = self.get_epoch_at_slot(slot, &epoch_schedule);

            if current_epoch != last_epoch {
                info!("Epoch boundary detected: {} -> {}", last_epoch, current_epoch);
                datapoint_info!(
                    "tip_router_epoch_boundary",
                    ("previous_epoch", last_epoch, i64),
                    ("current_epoch", current_epoch, i64)
                );

                if let Err(e) = self.create_snapshot(slot).await {
                    datapoint_error!(
                        "tip_router_snapshot_error",
                        ("slot", slot, i64),
                        ("error", format!("{:?}", e), String)
                    );
                    warn!("Failed to create snapshot: {:?}", e);
                }
                last_epoch = current_epoch;
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    async fn create_snapshot(&self, slot: Slot) -> Result<()> {
        datapoint_info!("tip_router_snapshot", ("slot", slot, i64), ("event", "start", String));

        let result = self.create_snapshot_internal(slot).await;

        match &result {
            Ok(_) => {
                datapoint_info!(
                    "tip_router_snapshot",
                    ("slot", slot, i64),
                    ("event", "success", String)
                );
            }
            Err(e) => {
                datapoint_error!(
                    "tip_router_snapshot",
                    ("slot", slot, i64),
                    ("error", format!("{:?}", e), String)
                );
            }
        }

        result
    }

    async fn create_snapshot_internal(&self, slot: Slot) -> Result<()> {
        let start_time = Instant::now();
        info!("Creating snapshot at slot {}", slot);

        // Track snapshot creation phases
        let mut current_phase = "initialization";
        self.report_snapshot_progress(slot, current_phase, 0.0)?;

        // Open blockstore with error context
        let blockstore = Blockstore::open(&self.blockstore_path).map_err(|e|
            anyhow!("Failed to open blockstore: {}", e)
        )?;
        let genesis_config = GenesisConfig::default();

        // Load bank forks
        current_phase = "loading_bank";
        self.report_snapshot_progress(slot, current_phase, 0.2)?;

        let (bank_forks, _leader_schedule_cache, _starting_snapshot_hashes) = bank_forks_utils
            ::load(
                &genesis_config,
                &blockstore,
                vec![self.output_dir.clone()],
                None,
                None,
                ProcessOptions::default(),
                None,
                None,
                None,
                None,
                Arc::new(AtomicBool::new(false))
            )
            .map_err(|e| anyhow!("Failed to load bank forks: {}", e))?;

        // Get bank for slot
        current_phase = "accessing_bank";
        self.report_snapshot_progress(slot, current_phase, 0.4)?;

        let bank = bank_forks
            .read()
            .unwrap()
            .get(slot)
            .ok_or_else(|| anyhow!("Failed to get bank at slot {}", slot))?;

        // Create snapshot archive
        current_phase = "creating_archive";
        self.report_snapshot_progress(slot, current_phase, 0.6)?;

        let archive_format = match self.compression.as_str() {
            "bzip2" => ArchiveFormat::TarBzip2,
            "gzip" => ArchiveFormat::TarGzip,
            "zstd" => ArchiveFormat::TarZstd,
            _ => ArchiveFormat::TarBzip2,
        };

        snapshot_bank_utils
            ::bank_to_full_snapshot_archive(
                &self.output_dir,
                &bank,
                Some(SnapshotVersion::default()),
                &self.output_dir,
                &self.output_dir,
                archive_format,
                NonZeroUsize::new(self.max_snapshots as usize).unwrap(),
                NonZeroUsize::new(self.max_snapshots as usize).unwrap()
            )
            .map_err(|e| anyhow!("Failed to create snapshot archive: {}", e))?;

        // Get snapshot file size
        current_phase = "finalizing";
        self.report_snapshot_progress(slot, current_phase, 0.9)?;

        let snapshot_path = self.output_dir.join(
            format!("snapshot-{}.tar.{}", slot, self.compression)
        );
        let snapshot_size = fs
            ::metadata(&snapshot_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let duration = start_time.elapsed();

        // Report final metrics
        datapoint_info!(
            "tip_router_snapshot_metrics",
            ("slot", slot, i64),
            ("duration_ms", duration.as_millis() as i64, i64),
            ("size_bytes", snapshot_size as i64, i64),
            ("compression", self.compression, String)
        );

        self.report_snapshot_progress(slot, "complete", 1.0)?;

        info!(
            "Snapshot created successfully - Size: {}MB, Duration: {:.2}s",
            snapshot_size / (1024 * 1024),
            duration.as_secs_f64()
        );

        Ok(())
    }

    pub async fn validate_snapshot(&self, slot: Slot) -> Result<()> {
        info!("Validating snapshot for slot {}", slot);

        let snapshot_archive_path = self.output_dir.join(
            format!("snapshot-{}.tar.{}", slot, self.compression)
        );

        if !snapshot_archive_path.exists() {
            return Err(anyhow!("Snapshot archive not found at {:?}", snapshot_archive_path));
        }

        let blockstore = Blockstore::open(&self.blockstore_path)?;
        let bank_hash = blockstore
            .get_bank_hash(slot)
            .ok_or_else(|| anyhow!("Bank hash not found for slot {}", slot))?;

        let temp_dir = tempfile::tempdir()?;
        let genesis_config = GenesisConfig::default(); // You might need to load this properly

        let (bank_forks, _leader_schedule_cache, _starting_snapshot_hashes) =
            bank_forks_utils::load_bank_forks(
                &genesis_config,
                &blockstore,
                vec![temp_dir.path().to_path_buf()],
                None,
                None,
                &Default::default(),
                None,
                None,
                None,
                Arc::new(AtomicBool::new(false))
            )?;

        let bank = bank_forks
            .read()
            .unwrap()
            .get(slot)
            .ok_or_else(|| anyhow!("Failed to load bank from snapshot at slot {}", slot))?;

        if bank.get_accounts_hash().map(|h| h.0) != Some(bank_hash) {
            return Err(anyhow!("Account hash mismatch for slot {}", slot));
        }

        Ok(())
    }

    pub async fn cleanup_old_snapshots(&self) -> Result<()> {
        info!("Cleaning up old snapshots");

        let mut snapshots: Vec<(Slot, PathBuf)> = fs
            ::read_dir(&self.output_dir)?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let path = entry.path();
                let file_name = path.file_name()?.to_str()?;
                if !file_name.starts_with("snapshot-") {
                    return None;
                }
                let slot = file_name
                    .strip_prefix("snapshot-")
                    .and_then(|s| s.split('.').next())
                    .and_then(|s| s.parse::<Slot>().ok())?; // Add ? here

                Some((slot, path)) // Remove the duplicate Some() below
            })
            .collect();

        // Sort by slot number, newest first
        snapshots.sort_by(|a, b| b.0.cmp(&a.0));

        // Keep only the specified number of latest snapshots
        for (_slot, path) in snapshots.iter().skip(self.max_snapshots as usize) {
            info!("Removing old snapshot: {:?}", path);
            if let Err(e) = fs::remove_file(path) {
                warn!("Failed to remove old snapshot {:?}: {}", path, e);
            }
        }

        Ok(())
    }

    fn report_snapshot_progress(&self, slot: Slot, phase: &str, progress: f32) -> Result<()> {
        let progress_info = SnapshotProgress {
            slot,
            phase: phase.to_string(),
            progress,
            message: format!("Creating snapshot - phase: {}", phase),
        };

        self.report_progress(progress_info);

        datapoint_info!(
            "tip_router_snapshot_progress",
            ("slot", slot, i64),
            ("phase", phase, String),
            ("progress", progress as f64, f64)
        );

        Ok(())
    }

    fn get_epoch_at_slot(&self, slot: Slot, epoch_schedule: &EpochSchedule) -> u64 {
        epoch_schedule.get_epoch(slot)
    }

    pub fn get_output_dir(&self) -> &PathBuf {
        &self.output_dir
    }
}

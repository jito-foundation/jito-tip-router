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
        info!("Creating snapshot at slot {}", slot);

        let blockstore = Blockstore::open(&self.blockstore_path)?;
        let genesis_config = GenesisConfig::default(); // You might need to load this properly

        let (bank_forks, _leader_schedule_cache, _starting_snapshot_hashes) =
            bank_forks_utils::load(
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
            )?;

        let bank = bank_forks
            .read()
            .unwrap()
            .get(slot)
            .ok_or_else(|| anyhow!("Failed to get bank at slot {}", slot))?;

        let archive_format = match self.compression.as_str() {
            "bzip2" => ArchiveFormat::TarBzip2,
            "gzip" => ArchiveFormat::TarGzip,
            "zstd" => ArchiveFormat::TarZstd,
            _ => ArchiveFormat::TarBzip2,
        };

        snapshot_bank_utils::bank_to_full_snapshot_archive(
            &self.output_dir, // bank_snapshots_dir
            &bank,
            Some(SnapshotVersion::default()), // snapshot_version needs to be Option<SnapshotVersion>
            &self.output_dir, // full_snapshot_archives_dir
            &self.output_dir, // incremental_snapshot_archives_dir
            archive_format,
            NonZeroUsize::new(self.max_snapshots as usize).unwrap(),
            NonZeroUsize::new(self.max_snapshots as usize).unwrap()
        )?;

        self.report_progress(SnapshotProgress {
            slot,
            phase: "snapshot_creation".to_string(),
            progress: 1.0,
            message: "Snapshot created successfully".to_string(),
        });

        Ok(())
    }

    async fn validate_snapshot(&self, slot: Slot) -> Result<()> {
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

    async fn cleanup_old_snapshots(&self) -> Result<()> {
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

    async fn create_stake_meta(&self, slot: Slot) -> Result<()> {
        info!("Creating stake meta for slot {}", slot);
        datapoint_info!("tip_router_stake_meta", ("slot", slot, i64), ("event", "start", String));

        // Load bank from snapshot
        let blockstore = Blockstore::open(&self.blockstore_path)?;
        let genesis_config = GenesisConfig::default();

        let (bank_forks, _, _) = bank_forks_utils::load(
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
        )?;

        let bank = bank_forks
            .read()
            .unwrap()
            .get(slot)
            .ok_or_else(|| anyhow!("Failed to get bank at slot {}", slot))?;

        // Get all vote accounts
        let vote_accounts = bank.vote_accounts();
        let mut validator_stake_meta = HashMap::new();

        // First, get all stake accounts once
        if let Ok(accounts) = bank.get_program_accounts(&solana_stake_program::id(), None) {
            // Process all stake accounts first
            for (stake_pubkey, account) in accounts {
                if
                    let Ok(stake_state) =
                        solana_stake_program::stake_state::StakeState::deserialize(
                            &account.data(),
                            account.owner()
                        )
                {
                    if
                        let solana_stake_program::stake_state::StakeState::Stake(_, stake) =
                            stake_state
                    {
                        let voter_pubkey = stake.delegation.voter_pubkey.to_string();
                        let lamports = account.lamports();

                        // Get or create the validator entry
                        validator_stake_meta
                            .entry(voter_pubkey)
                            .and_modify(|meta: &mut ValidatorStakeMeta| {
                                meta.total_stake += lamports;
                                meta.stake_accounts.insert(
                                    stake_pubkey.to_string(),
                                    StakeMetaAccount {
                                        lamports,
                                        owner: account.owner().to_string(),
                                        stake_authority: None,
                                        withdraw_authority: None,
                                    }
                                );
                            })
                            .or_insert_with(|| {
                                let mut stake_accounts = HashMap::new();
                                stake_accounts.insert(stake_pubkey.to_string(), StakeMetaAccount {
                                    lamports,
                                    owner: account.owner().to_string(),
                                    stake_authority: None,
                                    withdraw_authority: None,
                                });
                                ValidatorStakeMeta {
                                    vote_account: stake.delegation.voter_pubkey.to_string(),
                                    identity: String::new(), // Will be filled in next loop
                                    commission: 0, // Will be filled in next loop
                                    stake_accounts,
                                    total_stake: lamports,
                                }
                            });
                    }
                }
            }
        }

        // Now fill in the validator identity and commission information
        for (vote_pubkey, (validator_identity, vote_account)) in vote_accounts.as_ref() {
            if let Some(meta) = validator_stake_meta.get_mut(&vote_pubkey.to_string()) {
                meta.identity = validator_identity.to_string();
                meta.commission = vote_account.commission;
            }
        }

        // Write to file
        let meta_path = self.output_dir.join(format!("stake-meta-{}.json", slot));
        fs::write(&meta_path, serde_json::to_string_pretty(&validator_stake_meta)?)?;

        info!("Stake meta created at {:?}", meta_path);
        datapoint_info!(
            "tip_router_stake_meta",
            ("slot", slot, i64),
            ("event", "complete", String)
        );

        Ok(())
    }

    async fn create_merkle_trees(&self, slot: Slot) -> Result<()> {
        info!("Creating merkle trees for slot {}", slot);
        datapoint_info!("tip_router_merkle_trees", ("slot", slot, i64), ("event", "start", String));
        // TODO: Implement merkle tree creation
        Ok(())
    }

    async fn create_meta_merkle_tree(&self, slot: Slot) -> Result<()> {
        info!("Creating meta merkle tree for slot {}", slot);
        datapoint_info!(
            "tip_router_meta_merkle_tree",
            ("slot", slot, i64),
            ("event", "start", String)
        );
        // TODO: Implement meta merkle tree creation
        Ok(())
    }

    async fn upload_meta_merkle_root(&self, slot: Slot) -> Result<()> {
        info!("Uploading meta merkle root for slot {}", slot);
        datapoint_info!("tip_router_upload_root", ("slot", slot, i64), ("event", "start", String));
        // TODO: Implement NCN upload
        Ok(())
    }

    fn get_epoch_at_slot(&self, slot: Slot, epoch_schedule: &EpochSchedule) -> u64 {
        epoch_schedule.get_epoch(slot)
    }
}

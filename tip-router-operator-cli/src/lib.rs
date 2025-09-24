#![allow(clippy::arithmetic_side_effects)]
pub mod ledger_utils;
pub mod stake_meta_generator;
pub mod tip_router;
pub use crate::cli::{Cli, Commands};
pub mod arg_matches;
pub mod backup_snapshots;
pub mod claim;
pub mod cli;
pub mod distribution_meta;
pub mod load_and_process_ledger;
pub mod priority_fees;
pub mod process_epoch;
pub mod reclaim;
pub mod restaking;
pub mod rpc_utils;
pub mod solana_cli;
pub mod submit;
pub mod tip_distribution_stats;
pub mod tx_utils;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use borsh::BorshSerialize;
use cli::SnapshotPaths;
use jito_tip_payment_sdk::{
    CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2,
    TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6,
    TIP_ACCOUNT_SEED_7,
};
use ledger_utils::get_bank_from_ledger;
use log::info;
use meta_merkle_tree::generated_merkle_tree::StakeMetaCollection;
use meta_merkle_tree::{
    generated_merkle_tree::GeneratedMerkleTreeCollection, meta_merkle_tree::MetaMerkleTree,
};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_metrics::{datapoint_error, datapoint_info};
use solana_runtime::bank::Bank;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;
use solana_sdk::pubkey::Pubkey;
use stake_meta_generator::generate_stake_meta_collection;

#[derive(Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Default for Version {
    fn default() -> Self {
        Self {
            major: env!("CARGO_PKG_VERSION_MAJOR").parse().unwrap(),
            minor: env!("CARGO_PKG_VERSION_MINOR").parse().unwrap(),
            patch: env!("CARGO_PKG_VERSION_PATCH").parse().unwrap(),
        }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch,)
    }
}

#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum OperatorState {
    // Allows the operator to load from a snapshot created externally
    LoadBankFromSnapshot,
    CreateStakeMeta,
    CreateMerkleTreeCollection,
    CreateMetaMerkleTree,
    CastVote,
    WaitForNextEpoch,
}

pub fn stake_meta_file_name(epoch: u64) -> String {
    format!("{}_stake_meta_collection.json", epoch)
}

fn stake_meta_file_candidates(epoch: u64) -> [String; 2] {
    [
        format!("{}_stake_meta_collection.json", epoch),
        format!("{}-stake-meta-collection.json", epoch),
    ]
}

pub fn read_stake_meta_collection(epoch: u64, save_path: &Path) -> StakeMetaCollection {
    let stake_meta_file_candidates: &[String] = &stake_meta_file_candidates(epoch);

    let candidate_paths = stake_meta_file_candidates
        .iter()
        .map(|filename| {
            let path = save_path.join(filename);
            PathBuf::from(&path)
        })
        .collect::<Vec<_>>();

    let valid_stake_meta_file_names = candidate_paths
        .iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let stake_meta_file_name = valid_stake_meta_file_names
        .first()
        .expect("Failed to find a valid stake meta file");

    StakeMetaCollection::new_from_file(stake_meta_file_name).unwrap_or_else(|_| {
        panic!(
            "Failed to load stake meta collection from file: {}",
            stake_meta_file_name.display()
        )
    })
}

pub fn merkle_tree_collection_file_name(epoch: u64) -> String {
    format!("{}_merkle_tree_collection.json", epoch)
}

fn merkle_tree_collection_file_candidates(epoch: u64) -> [String; 2] {
    [
        format!("{}_merkle_tree_collection.json", epoch),
        format!("{}-merkle-tree-collection.json", epoch),
    ]
}

pub fn read_merkle_tree_collection(epoch: u64, save_path: &Path) -> GeneratedMerkleTreeCollection {
    let merkle_tree_file_candidates: &[String] = &merkle_tree_collection_file_candidates(epoch);

    let candidate_paths = merkle_tree_file_candidates
        .iter()
        .map(|filename| {
            let path = save_path.join(filename);
            PathBuf::from(&path)
        })
        .collect::<Vec<_>>();

    let valid_merkle_tree_file_names = candidate_paths
        .iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let merkle_tree_file_name = valid_merkle_tree_file_names
        .first()
        .expect("Failed to find a valid merkle tree file");

    GeneratedMerkleTreeCollection::new_from_file(merkle_tree_file_name).unwrap_or_else(|_| {
        panic!(
            "Failed to load merkle tree collection from file: {}",
            merkle_tree_file_name.display()
        )
    })
}

pub fn meta_merkle_tree_file_name(epoch: u64) -> String {
    format!("{}_meta_merkle_tree.json", epoch)
}

pub fn meta_merkle_tree_file_candidates(epoch: u64) -> [String; 2] {
    [
        format!("{}_meta_merkle_tree.json", epoch),
        format!("{}-meta-merkle-tree.json", epoch),
    ]
}

pub fn meta_merkle_tree_path(epoch: u64, save_path: &Path) -> PathBuf {
    let meta_merkle_file_candidates: &[String] = &meta_merkle_tree_file_candidates(epoch);

    let candidate_paths = meta_merkle_file_candidates
        .iter()
        .map(|filename| {
            let path = save_path.join(filename);
            PathBuf::from(&path)
        })
        .collect::<Vec<_>>();

    let meta_merkle_tree_filenames = candidate_paths
        .iter()
        .filter(|path| path.exists())
        .collect::<Vec<_>>();

    let meta_merkle_tree_filename = meta_merkle_tree_filenames
        .first()
        .expect("Failed to find a valid meta merkle tree file");

    PathBuf::from(meta_merkle_tree_filename)
}

// STAGE 1 LoadBankFromSnapshot
pub fn load_bank_from_snapshot(cli: Cli, slot: u64, save_snapshot: bool) -> Arc<Bank> {
    let SnapshotPaths {
        ledger_path,
        account_paths,
        full_snapshots_path,
        incremental_snapshots_path: _,
        backup_snapshots_dir,
    } = cli.get_snapshot_paths();

    get_bank_from_ledger(
        cli.operator_address,
        &ledger_path,
        account_paths,
        full_snapshots_path,
        backup_snapshots_dir.clone(),
        &slot,
        save_snapshot,
        backup_snapshots_dir,
        &cli.cluster,
    )
}

// STAGE 2 CreateStakeMeta
#[allow(clippy::too_many_arguments)]
pub fn create_stake_meta(
    operator_address: String,
    epoch: u64,
    bank: &Arc<Bank>,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    save_path: &Path,
    save: bool,
    cluster: &str,
) -> StakeMetaCollection {
    let start = Instant::now();

    info!("Generating stake_meta_collection object...");
    let stake_meta_coll = match generate_stake_meta_collection(
        bank,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_payment_program_id,
    ) {
        Ok(stake_meta) => stake_meta,
        Err(e) => {
            let error_str = format!("{:?}", e);
            datapoint_error!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address, String),
                ("epoch", epoch, i64),
                ("status", "error", String),
                ("error", error_str, String),
                ("state", "stake_meta_generation", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64),
                "cluster" => cluster,
            );
            panic!("{}", error_str);
        }
    };

    info!(
        "Created StakeMetaCollection:\n - epoch: {:?}\n - slot: {:?}\n - num stake metas: {:?}\n - bank_hash: {:?}",
        stake_meta_coll.epoch,
        stake_meta_coll.slot,
        stake_meta_coll.stake_metas.len(),
        stake_meta_coll.bank_hash
    );
    if save {
        // Note: We have the epoch come before the file name so ordering is neat on a machine
        //  with multiple epochs saved.
        let file = save_path.join(stake_meta_file_name(epoch));
        stake_meta_coll.write_to_file(&file);
    }

    datapoint_info!(
        "tip_router_cli.process_epoch",
        ("operator_address", operator_address, String),
        ("state", "create_stake_meta", String),
        ("step", 2, i64),
        ("epoch", stake_meta_coll.epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64),
        "cluster" => cluster,
    );
    stake_meta_coll
}

// STAGE 3 CreateMerkleTreeCollection
#[allow(clippy::too_many_arguments)]
pub fn create_merkle_tree_collection(
    operator_address: String,
    tip_router_program_id: &Pubkey,
    stake_meta_collection: StakeMetaCollection,
    epoch: u64,
    ncn_address: &Pubkey,
    protocol_fee_bps: u64,
    pf_distribution_protocol_fee_bps: u64,
    save_path: &Path,
    save: bool,
    cluster: &str,
) -> GeneratedMerkleTreeCollection {
    let start = Instant::now();

    // Generate merkle tree collection
    let merkle_tree_coll = match GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
        stake_meta_collection,
        ncn_address,
        epoch,
        protocol_fee_bps,
        pf_distribution_protocol_fee_bps,
        tip_router_program_id,
    ) {
        Ok(merkle_tree_coll) => merkle_tree_coll,
        Err(e) => {
            let error_str = format!("{:?}", e);
            datapoint_error!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address, String),
                ("epoch", epoch, i64),
                ("status", "error", String),
                ("error", error_str, String),
                ("state", "merkle_tree_generation", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64),
                "cluster" => cluster
            );
            panic!("{}", error_str);
        }
    };
    info!(
        "Created GeneratedMerkleTreeCollection:\n - epoch: {:?}\n - slot: {:?}\n - num generated merkle trees: {:?}\n - bank_hash: {:?}",
        merkle_tree_coll.epoch,
        merkle_tree_coll.slot,
        merkle_tree_coll.generated_merkle_trees.len(),
        merkle_tree_coll.bank_hash
    );

    if save {
        // Note: We have the epoch come before the file name so ordering is neat on a machine
        //  with multiple epochs saved.
        let file = save_path.join(merkle_tree_collection_file_name(epoch));
        match merkle_tree_coll.write_to_file(&file) {
            Ok(_) => {}
            Err(e) => {
                let error_str = format!("{:?}", e);
                datapoint_error!(
                    "tip_router_cli.process_epoch",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch, i64),
                    ("status", "error", String),
                    ("error", error_str, String),
                    ("state", "merkle_root_file_write", String),
                    ("duration_ms", start.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster
                );
                panic!("{:?}", e);
            }
        }
    }
    datapoint_info!(
        "tip_router_cli.process_epoch",
        ("operator_address", operator_address, String),
        ("state", "meta_merkle_tree_creation", String),
        ("step", 3, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64),
        "cluster" => cluster
    );
    merkle_tree_coll
}

// STAGE 4 CreateMetaMerkleTree
pub fn create_meta_merkle_tree(
    operator_address: String,
    merkle_tree_collection: GeneratedMerkleTreeCollection,
    epoch: u64,
    save_path: &Path,
    save: bool,
    cluster: &str,
) -> MetaMerkleTree {
    let start = Instant::now();
    let meta_merkle_tree =
        match MetaMerkleTree::new_from_generated_merkle_tree_collection(merkle_tree_collection) {
            Ok(meta_merkle_tree) => meta_merkle_tree,
            Err(e) => {
                let error_str = format!("{:?}", e);
                datapoint_error!(
                    "tip_router_cli.create_meta_merkle_tree",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch, i64),
                    ("status", "error", String),
                    ("error", error_str, String),
                    ("state", "merkle_tree_generation", String),
                    ("duration_ms", start.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                panic!("{}", error_str);
            }
        };

    info!(
        "Created MetaMerkleTree:\n - num nodes: {:?}\n - merkle root: {:?}",
        meta_merkle_tree.num_nodes, meta_merkle_tree.merkle_root
    );

    if save {
        // Note: We have the epoch come before the file name so ordering is neat on a machine
        //  with multiple epochs saved.
        let file = save_path.join(meta_merkle_tree_file_name(epoch));
        match meta_merkle_tree.write_to_file(&file) {
            Ok(_) => {}
            Err(e) => {
                let error_str = format!("{:?}", e);
                datapoint_error!(
                    "tip_router_cli.create_meta_merkle_tree",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch, i64),
                    ("status", "error", String),
                    ("error", error_str, String),
                    ("state", "merkle_root_file_write", String),
                    ("duration_ms", start.elapsed().as_millis() as i64, i64),
                    "cluster" => cluster,
                );
                panic!("{:?}", e);
            }
        }
    }

    datapoint_info!(
        "tip_router_cli.create_meta_merkle_tree",
        ("operator_address", operator_address, String),
        ("state", "meta_merkle_tree_creation", String),
        ("step", 4, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64),
        "cluster" => cluster,
    );

    meta_merkle_tree
}

#[derive(Debug)]
pub enum MerkleRootError {
    StakeMetaGeneratorError(String),
    MerkleRootGeneratorError(String),
    MerkleTreeError(String),
}

// TODO where did these come from?
pub struct TipPaymentPubkeys {
    _config_pda: Pubkey,
    tip_pdas: Vec<Pubkey>,
}

#[derive(BorshSerialize, Clone, Debug, Default)]
pub struct TipAccountConfig {
    pub authority: Pubkey,
    pub protocol_fee_bps: u64,
    pub bump: u8,
}

fn derive_tip_payment_pubkeys(program_id: &Pubkey) -> TipPaymentPubkeys {
    let config_pda = Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], program_id).0;
    let tip_pda_0 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], program_id).0;
    let tip_pda_1 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], program_id).0;
    let tip_pda_2 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], program_id).0;
    let tip_pda_3 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], program_id).0;
    let tip_pda_4 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], program_id).0;
    let tip_pda_5 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], program_id).0;
    let tip_pda_6 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], program_id).0;
    let tip_pda_7 = Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], program_id).0;

    TipPaymentPubkeys {
        _config_pda: config_pda,
        tip_pdas: vec![
            tip_pda_0, tip_pda_1, tip_pda_2, tip_pda_3, tip_pda_4, tip_pda_5, tip_pda_6, tip_pda_7,
        ],
    }
}

fn get_validator_cmdline() -> Result<String> {
    let output = Command::new("pgrep").arg("solana-validator").output()?;

    let pid = String::from_utf8_lossy(&output.stdout).trim().to_string();

    let cmdline = fs::read_to_string(format!("/proc/{}/cmdline", pid))?;

    Ok(cmdline.replace('\0', " "))
}

pub fn emit_solana_validator_args() -> std::result::Result<(), anyhow::Error> {
    // Find solana-validator process and get its command line args
    let validator_cmdline = match get_validator_cmdline() {
        Ok(cmdline) => cmdline,
        Err(_) => return Err(anyhow::anyhow!("Validator process not found")),
    };

    let validator_config: Vec<String> = validator_cmdline
        .split_whitespace()
        .map(String::from)
        .collect();

    if validator_config.is_empty() {
        return Err(anyhow::anyhow!("Validator process not found"));
    }

    let mut limit_ledger_size = None;
    let mut full_snapshot_interval = None;
    let mut max_full_snapshots = None;
    let mut incremental_snapshot_path = None;
    let mut incremental_snapshot_interval = None;
    let mut max_incremental_snapshots = None;

    for (i, arg) in validator_config.iter().enumerate() {
        match arg.as_str() {
            "--limit-ledger-size" => {
                if let Some(value) = validator_config.get(i + 1) {
                    limit_ledger_size = Some(value.clone());
                }
            }
            "--full-snapshot-interval-slots" => {
                if let Some(value) = validator_config.get(i + 1) {
                    full_snapshot_interval = Some(value.clone());
                }
            }
            "--maximum-full-snapshots-to-retain" => {
                if let Some(value) = validator_config.get(i + 1) {
                    max_full_snapshots = Some(value.clone());
                }
            }
            "--incremental-snapshot-archive-path" => {
                if let Some(value) = validator_config.get(i + 1) {
                    incremental_snapshot_path = Some(value.clone());
                }
            }
            "--incremental-snapshot-interval-slots" => {
                if let Some(value) = validator_config.get(i + 1) {
                    incremental_snapshot_interval = Some(value.clone());
                }
            }
            "--maximum-incremental-snapshots-to-retain" => {
                if let Some(value) = validator_config.get(i + 1) {
                    max_incremental_snapshots = Some(value.clone());
                }
            }
            _ => {}
        }
    }

    datapoint_info!(
        "tip_router_cli.validator_config",
        (
            "limit_ledger_size",
            limit_ledger_size.unwrap_or_default(),
            String
        ),
        (
            "full_snapshot_interval",
            full_snapshot_interval.unwrap_or_default(),
            String
        ),
        (
            "max_full_snapshots",
            max_full_snapshots.unwrap_or_default(),
            String
        ),
        (
            "incremental_snapshot_path",
            incremental_snapshot_path.unwrap_or_default(),
            String
        ),
        (
            "incremental_snapshot_interval",
            incremental_snapshot_interval.unwrap_or_default(),
            String
        ),
        (
            "max_incremental_snapshots",
            max_incremental_snapshots.unwrap_or_default(),
            String
        )
    );

    Ok(())
}

pub fn cleanup_tmp_files(snapshot_output_dir: &Path) -> std::result::Result<(), anyhow::Error> {
    // Fail if snapshot_output_dir is "/"
    if snapshot_output_dir == Path::new("/") {
        return Err(anyhow::anyhow!("snapshot_output_dir cannot be /"));
    }

    // Remove stake-meta.accounts directory
    let stake_meta_path = snapshot_output_dir.join("stake-meta.accounts");
    if stake_meta_path.exists() {
        if stake_meta_path.is_dir() {
            std::fs::remove_dir_all(&stake_meta_path)?;
        } else {
            std::fs::remove_file(&stake_meta_path)?;
        }
    }

    // Remove tmp* files/directories in snapshot dir
    for entry in std::fs::read_dir(snapshot_output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            if let Some(file_name_str) = file_name.to_str() {
                if file_name_str.starts_with("tmp") {
                    if path.is_dir() {
                        std::fs::remove_dir_all(path)?;
                    } else {
                        std::fs::remove_file(path)?;
                    }
                }
            }
        }
    }

    // Remove /tmp/.tmp* files/directories
    let tmp_dir = PathBuf::from("/tmp");
    if tmp_dir.exists() {
        for entry in std::fs::read_dir(&tmp_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(file_name) = path.file_name() {
                if let Some(file_name_str) = file_name.to_str() {
                    if file_name_str.starts_with(".tmp") {
                        if path.is_dir() {
                            std::fs::remove_dir_all(path)?;
                        } else {
                            std::fs::remove_file(path)?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub async fn get_epoch_percentage(client: &RpcClient) -> anyhow::Result<f64> {
    let current_slot = client.get_slot().await? as f64;
    let epoch_percentage =
        (current_slot % DEFAULT_SLOTS_PER_EPOCH as f64) / DEFAULT_SLOTS_PER_EPOCH as f64;
    Ok(epoch_percentage)
}

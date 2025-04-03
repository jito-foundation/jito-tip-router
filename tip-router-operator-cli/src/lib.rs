#![allow(clippy::arithmetic_side_effects)]
pub mod ledger_utils;
pub mod stake_meta_generator;
pub mod tip_router;
pub use crate::cli::{Cli, Commands};
pub mod arg_matches;
pub mod backup_snapshots;
pub mod claim;
pub mod cli;
pub mod load_and_process_ledger;
pub mod priority_fee_utils;
pub mod process_epoch;
pub mod rpc_utils;
pub mod submit;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use anchor_lang::prelude::*;
use cli::SnapshotPaths;
use ellipsis_client::EllipsisClient;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::TipDistributionAccount;
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
use priority_fee_utils::{get_priority_fees_for_epoch, LeaderEpochPriorityFees};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_runtime::bank::Bank;
use solana_sdk::{account::AccountSharedData, pubkey::Pubkey};
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
    CreateLeaderPriorityFeeMap,
    CreateStakeMeta,
    CreateMerkleTreeCollection,
    CreateMetaMerkleTree,
    CastVote,
    WaitForNextEpoch,
}

pub fn leader_priority_fees_file_name(epoch: u64) -> String {
    format!("{}_leader_priority_fees.json", epoch)
}

pub fn stake_meta_file_name(epoch: u64) -> String {
    format!("{}_stake_meta_collection.json", epoch)
}

pub fn merkle_tree_collection_file_name(epoch: u64) -> String {
    format!("{}_merkle_tree_collection.json", epoch)
}

pub fn meta_merkle_tree_file_name(epoch: u64) -> String {
    format!("{}_meta_merkle_tree.json", epoch)
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
    )
}

// STAGE 2 generate the priority fees per leader for a given epoch
pub async fn create_leader_priority_fee_map(
    rpc_client: &EllipsisClient,
    operator_address: String,
    epoch: u64,
    save_path: &Path,
    save: bool,
) -> LeaderEpochPriorityFees {
    let start = Instant::now();
    let leader_priority_fees = match get_priority_fees_for_epoch(rpc_client, epoch).await {
        Ok(leader_priority_fees) => leader_priority_fees,
        Err(e) => {
            let error_str = format!("{:?}", e);
            datapoint_error!(
                "tip_router_cli.create_leader_priority_fee_map",
                ("operator_address", operator_address, String),
                ("epoch", epoch, i64),
                ("status", "error", String),
                ("error", error_str, String),
                ("state", "create_leader_priority_fee_map", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
            panic!("{}", error_str);
        }
    };
    if save {
        let file = save_path.join(leader_priority_fees_file_name(epoch));
        match leader_priority_fees.write_to_file(&file) {
            Ok(_) => {}
            Err(e) => {
                let error_str = format!("{:?}", e);
                datapoint_error!(
                    "tip_router_cli.create_leader_priority_fee_map",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch, i64),
                    ("status", "error", String),
                    ("error", error_str, String),
                    ("state", "write_to_file", String),
                    ("duration_ms", start.elapsed().as_millis() as i64, i64)
                );
            }
        };
    }
    leader_priority_fees
}

// STAGE 3 CreateStakeMeta
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
    leader_priority_fees_map: &HashMap<String, u64>,
) -> StakeMetaCollection {
    let start = Instant::now();

    info!("Generating stake_meta_collection object...");
    let stake_meta_coll = match generate_stake_meta_collection(
        bank,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_payment_program_id,
        leader_priority_fees_map,
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
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
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
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );
    stake_meta_coll
}

// STAGE 4 CreateMerkleTreeCollection
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
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
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
                    ("duration_ms", start.elapsed().as_millis() as i64, i64)
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
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );
    merkle_tree_coll
}

// STAGE 5 CreateMetaMerkleTree
pub fn create_meta_merkle_tree(
    operator_address: String,
    merkle_tree_collection: GeneratedMerkleTreeCollection,
    epoch: u64,
    save_path: &Path,
    save: bool,
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
                    ("duration_ms", start.elapsed().as_millis() as i64, i64)
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
                    ("duration_ms", start.elapsed().as_millis() as i64, i64)
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
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
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

#[derive(Clone, Debug, AnchorSerialize, AnchorDeserialize)]
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

/// Convenience wrapper around [TipDistributionAccount]
pub struct TipDistributionAccountWrapper {
    pub tip_distribution_account: TipDistributionAccount,
    pub account_data: AccountSharedData,
    pub tip_distribution_pubkey: Pubkey,
}

/// Convenience wrapper around [PriorityFeeDistributionAccount]
pub struct PriorityFeeDistributionAccountWrapper {
    pub priority_fee_distribution_account: PriorityFeeDistributionAccount,
    pub account_data: AccountSharedData,
    pub priority_fee_distribution_pubkey: Pubkey,
    pub total_prioity_fees: u64,
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

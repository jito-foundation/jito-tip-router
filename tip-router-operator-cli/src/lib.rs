pub mod ledger_utils;
pub mod stake_meta_generator;
pub mod tip_router;
pub use crate::cli::{Cli, Commands};
pub mod cli;
pub use crate::process_epoch::process_epoch;
pub mod arg_matches;
pub mod backup_snapshots;
mod load_and_process_ledger;
pub mod process_epoch;
pub mod submit;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use anchor_lang::prelude::*;
use jito_tip_distribution_sdk::TipDistributionAccount;
use jito_tip_payment_sdk::{
    CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2,
    TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6,
    TIP_ACCOUNT_SEED_7,
};
use log::info;
use meta_merkle_tree::{
    generated_merkle_tree::GeneratedMerkleTreeCollection, meta_merkle_tree::MetaMerkleTree,
};
use solana_metrics::datapoint_info;
use solana_sdk::{account::AccountSharedData, pubkey::Pubkey, slot_history::Slot};

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

#[allow(clippy::too_many_arguments)]
pub fn get_meta_merkle_root(
    ledger_path: &Path,
    account_paths: Vec<PathBuf>,
    full_snapshots_path: PathBuf,
    incremental_snapshots_path: PathBuf,
    desired_slot: &Slot,
    tip_distribution_program_id: &Pubkey,
    out_path: &str,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    operator_address: &Pubkey,
    epoch: u64,
    protocol_fee_bps: u64,
    snapshots_enabled: bool,
) -> std::result::Result<MetaMerkleTree, MerkleRootError> {
    let start = Instant::now();

    datapoint_info!(
        "tip_router_cli.get_meta_merkle_root",
        ("operator_address", operator_address.to_string(), String),
        ("state", "stake_meta_generation", String),
        ("step", 1, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );

    // cleanup tmp files - update with path where stake meta is written
    match cleanup_tmp_files(&incremental_snapshots_path.clone()) {
        Ok(_) => {}
        Err(e) => {
            datapoint_info!(
                "tip_router_cli.get_meta_merkle_root",
                ("operator_address", operator_address.to_string(), String),
                ("state", "cleanup_tmp_files", String),
                ("error", format!("{:?}", e), String),
                ("epoch", epoch, i64),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
        }
    }

    // Get stake meta collection
    let stake_meta_collection = stake_meta_generator::generate_stake_meta(
        operator_address,
        ledger_path,
        account_paths,
        full_snapshots_path,
        incremental_snapshots_path.clone(),
        desired_slot,
        tip_distribution_program_id,
        out_path,
        tip_payment_program_id,
        snapshots_enabled,
    )
    .map_err(|e| {
        MerkleRootError::StakeMetaGeneratorError(format!("Failed to generate stake meta: {:?}", e))
    })?;

    info!(
        "Created StakeMetaCollection:\n - epoch: {:?}\n - slot: {:?}\n - num stake metas: {:?}\n - bank_hash: {:?}",
        stake_meta_collection.epoch,
        stake_meta_collection.slot,
        stake_meta_collection.stake_metas.len(),
        stake_meta_collection.bank_hash
    );

    datapoint_info!(
        "tip_router_cli.get_meta_merkle_root",
        ("operator_address", operator_address.to_string(), String),
        ("state", "generated_merkle_tree_collection", String),
        ("step", 2, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );

    // Cleanup tmp files
    match cleanup_tmp_files(&incremental_snapshots_path) {
        Ok(_) => {}
        Err(e) => {
            datapoint_info!(
                "tip_router_cli.get_meta_merkle_root",
                ("operator_address", operator_address.to_string(), String),
                ("state", "cleanup_tmp_files", String),
                ("error", format!("{:?}", e), String),
                ("epoch", epoch, i64),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
        }
    }

    // Generate merkle tree collection
    let merkle_tree_coll = GeneratedMerkleTreeCollection::new_from_stake_meta_collection(
        stake_meta_collection,
        ncn_address,
        epoch,
        protocol_fee_bps,
    )
    .map_err(|_| {
        MerkleRootError::MerkleRootGeneratorError(
            "Failed to generate merkle tree collection".to_string(),
        )
    })?;

    info!(
        "Created GeneratedMerkleTreeCollection:\n - epoch: {:?}\n - slot: {:?}\n - num generated merkle trees: {:?}\n - bank_hash: {:?}",
        merkle_tree_coll.epoch,
        merkle_tree_coll.slot,
        merkle_tree_coll.generated_merkle_trees.len(),
        merkle_tree_coll.bank_hash
    );

    datapoint_info!(
        "tip_router_cli.get_meta_merkle_root",
        ("operator_address", operator_address.to_string(), String),
        ("state", "meta_merkle_tree_creation", String),
        ("step", 3, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );

    // Convert to MetaMerkleTree
    let meta_merkle_tree = MetaMerkleTree::new_from_generated_merkle_tree_collection(
        merkle_tree_coll,
    )
    .map_err(|e| {
        MerkleRootError::MerkleTreeError(format!("Failed to create meta merkle tree: {:?}", e))
    })?;

    info!(
        "Created MetaMerkleTree:\n - num nodes: {:?}\n - merkle root: {:?}",
        meta_merkle_tree.num_nodes, meta_merkle_tree.merkle_root
    );

    datapoint_info!(
        "tip_router_cli.get_meta_merkle_root",
        ("operator_address", operator_address.to_string(), String),
        ("state", "meta_merkle_tree_creation", String),
        ("step", 4, i64),
        ("epoch", epoch, i64),
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );

    Ok(meta_merkle_tree)
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

pub mod ledger_utils;
pub mod stake_meta_generator;
pub mod tip_router;
pub use crate::cli::{Cli, Commands};
pub mod claim;
pub mod cli;
pub use crate::process_epoch::process_epoch;
pub mod backup_snapshots;
pub mod process_epoch;
pub mod rpc_utils;
pub mod submit;

use std::path::{Path, PathBuf};
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
use solana_metrics::{datapoint_error, datapoint_info};
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
    meta_merkle_tree_dir: &PathBuf,
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

    // Get stake meta collection
    let stake_meta_collection = stake_meta_generator::generate_stake_meta(
        operator_address,
        ledger_path,
        account_paths,
        full_snapshots_path,
        incremental_snapshots_path,
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

    // TODO: Hide this behind a flag when the process gets split up into the various stages and
    //  checkpoints.

    // Write GeneratedMerkleTreeCollection to disk. Required for Claiming
    let merkle_tree_coll_path =
        meta_merkle_tree_dir.join(format!("merkle_tree_coll_{}.json", epoch));
    let generated_merkle_tree_col_json = match serde_json::to_string(&merkle_tree_coll) {
        Ok(json) => json,
        Err(e) => {
            datapoint_error!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address.to_string(), String),
                ("epoch", epoch, i64),
                ("status", "error", String),
                ("error", format!("{:?}", e), String),
                ("state", "merkle_root_serialization", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
            return Err(MerkleRootError::MerkleRootGeneratorError(
                "Failed to serialize merkle tree collection",
            ));
        }
    };

    if let Err(e) = std::fs::write(&merkle_tree_coll_path, generated_merkle_tree_col_json) {
        datapoint_error!(
            "tip_router_cli.process_epoch",
            ("operator_address", operator_address.to_string(), String),
            ("epoch", epoch, i64),
            ("status", "error", String),
            ("error", format!("{:?}", e), String),
            ("state", "merkle_root_file_write", String),
            ("duration_ms", start.elapsed().as_millis() as i64, i64)
        );
        // TODO: propogate error
        return Err(MerkleRootError::MerkleRootGeneratorError(
            "Failed to write meta merkle tree to file",
        ));
    }

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

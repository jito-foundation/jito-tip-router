use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use ellipsis_client::EllipsisClient;
use log::{error, info};
use meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection};
use solana_metrics::datapoint_error;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_runtime::bank::Bank;
use solana_sdk::{epoch_info::EpochInfo, pubkey::Pubkey};
use tokio::time;

use crate::{
    backup_snapshots::SnapshotInfo, create_merkle_tree_collection, create_meta_merkle_tree,
    create_stake_meta, ledger_utils::get_bank_from_snapshot_at_slot, load_bank_from_snapshot,
    merkle_tree_collection_file_name, stake_meta_file_name, Cli, OperatorState, PROTOCOL_FEE_BPS,
};

const MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS: u64 = 1200; // Experimentally determined
const OPTIMAL_INCREMENTAL_SNAPSHOT_SLOT_RANGE: u64 = 800; // Experimentally determined

pub async fn wait_for_next_epoch(rpc_client: &RpcClient, current_epoch: u64) -> EpochInfo {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        let new_epoch_info = match rpc_client.get_epoch_info().await {
            Ok(info) => info,
            Err(e) => {
                error!("Error getting epoch info: {:?}", e);
                continue;
            }
        };

        if new_epoch_info.epoch > current_epoch {
            info!(
                "New epoch detected: {} -> {}",
                current_epoch, new_epoch_info.epoch
            );
            return new_epoch_info;
        }
    }
}

pub async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<(u64, u64)> {
    let epoch_info = rpc_client.get_epoch_info().await?;
    calc_prev_epoch_and_final_slot(&epoch_info)
}

pub fn calc_prev_epoch_and_final_slot(epoch_info: &EpochInfo) -> Result<(u64, u64)> {
    let current_slot = epoch_info.absolute_slot;
    let slot_index = epoch_info.slot_index;

    // Handle case where we're in the first epoch
    if current_slot < slot_index {
        return Ok((0, 0));
    }

    let epoch_start_slot = current_slot
        .checked_sub(slot_index)
        .ok_or_else(|| anyhow::anyhow!("epoch_start_slot subtraction overflow"))?;
    let previous_epoch_final_slot = epoch_start_slot.saturating_sub(1);
    let previous_epoch = epoch_info.epoch.saturating_sub(1);

    Ok((previous_epoch, previous_epoch_final_slot))
}

/// Wait for the optimal incremental snapshot to be available to speed up full snapshot generation
/// Automatically returns after MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS seconds
pub async fn wait_for_optimal_incremental_snapshot(
    incremental_snapshots_dir: PathBuf,
    target_slot: u64,
) -> Result<()> {
    let mut interval = time::interval(Duration::from_secs(1));
    let mut ticks = 0;

    while ticks < MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS {
        let dir_entries = std::fs::read_dir(&incremental_snapshots_dir)?;

        for entry in dir_entries {
            if let Some(snapshot_info) = SnapshotInfo::from_path(entry?.path()) {
                if target_slot - OPTIMAL_INCREMENTAL_SNAPSHOT_SLOT_RANGE < snapshot_info.end_slot
                    && snapshot_info.end_slot <= target_slot
                {
                    return Ok(());
                }
            }
        }

        interval.tick().await;
        ticks += 1;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn loop_stages(
    rpc_client: EllipsisClient,
    cli: Cli,
    starting_stage: OperatorState,
    override_target_slot: Option<u64>,
    tip_router_program_id: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    enable_snapshots: bool,
    save_stages: bool,
) -> Result<()> {
    let mut current_epoch_info = rpc_client.get_epoch_info().await?;

    // Track runs that are starting right at the beginning of a new epoch
    let operator_address = cli.operator_address.clone();
    let mut stage = starting_stage;
    let mut bank: Option<Arc<Bank>> = None;
    let mut stake_meta_collection: Option<StakeMetaCollection> = None;
    let mut merkle_tree_collection: Option<GeneratedMerkleTreeCollection> = None;
    let mut epoch_to_process = current_epoch_info.epoch.saturating_sub(1);
    let mut slot_to_process = if let Some(slot) = override_target_slot {
        slot
    } else {
        let (_, prev_slot) = calc_prev_epoch_and_final_slot(&current_epoch_info)?;
        prev_slot
    };
    loop {
        match stage {
            OperatorState::LoadBankFromSnapshot => {
                bank = Some(load_bank_from_snapshot(
                    cli.clone(),
                    slot_to_process,
                    enable_snapshots,
                ));
                // Transition to the next stage
                stage = OperatorState::CreateStakeMeta;
            }
            OperatorState::CreateStakeMeta => {
                let start = Instant::now();
                if bank.is_none() {
                    // TODO: REVIEW should we expect snapshots to be in the save path rather than
                    // the typical validator snapshots path? This would save the fight from the
                    // validator process removing snapshots. We'd have to also update the snapshot
                    // process and CLI to handle

                    // TODO: DRY up these paths from the Cli arguments (duplicate code in load_bank_from_snapshot)
                    let ledger_path = cli.ledger_path.clone();
                    let account_paths = None;
                    let account_paths =
                        account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
                    let full_snapshots_path = cli.full_snapshots_path.clone();
                    let full_snapshots_path =
                        full_snapshots_path.map_or(ledger_path.clone(), |path| path);
                    let maybe_bank = get_bank_from_snapshot_at_slot(
                        slot_to_process,
                        &full_snapshots_path,
                        &full_snapshots_path,
                        account_paths,
                        ledger_path.as_path(),
                    );
                    match maybe_bank {
                        Ok(some_bank) => bank = Some(Arc::new(some_bank)),
                        Err(e) => {
                            datapoint_error!(
                                "tip_router_cli.create_stake_meta",
                                ("operator_address", operator_address, String),
                                ("epoch", epoch_to_process, i64),
                                ("status", "error", String),
                                ("error", e.to_string(), String),
                                ("state", "create_stake_meta", String),
                                ("duration_ms", start.elapsed().as_millis() as i64, i64)
                            );
                            panic!("{}", e.to_string());
                        }
                    }
                }
                stake_meta_collection = Some(create_stake_meta(
                    operator_address.clone(),
                    epoch_to_process,
                    bank.as_ref().expect("Bank was not set"),
                    tip_distribution_program_id,
                    tip_payment_program_id,
                    &cli.save_path,
                    save_stages,
                ));
                // we should be able to safely drop the bank in this loop
                bank = None;
                // Transition to the next stage
                stage = OperatorState::CreateMerkleTreeCollection;
            }
            OperatorState::CreateMerkleTreeCollection => {
                let some_stake_meta_collection = match stake_meta_collection.to_owned() {
                    Some(collection) => collection,
                    None => {
                        let file = cli.save_path.join(stake_meta_file_name(epoch_to_process));
                        StakeMetaCollection::new_from_file(&file)?
                    }
                };

                // Generate the merkle tree collection
                merkle_tree_collection = Some(create_merkle_tree_collection(
                    cli.operator_address.clone(),
                    tip_router_program_id,
                    some_stake_meta_collection,
                    epoch_to_process,
                    ncn_address,
                    PROTOCOL_FEE_BPS,
                    &cli.save_path,
                    save_stages,
                ));

                stake_meta_collection = None;
                // Transition to the next stage
                stage = OperatorState::CreateMetaMerkleTree;
            }
            OperatorState::CreateMetaMerkleTree => {
                let some_merkle_tree_collection = match merkle_tree_collection.to_owned() {
                    Some(collection) => collection,
                    None => {
                        let file = cli
                            .save_path
                            .join(merkle_tree_collection_file_name(epoch_to_process));
                        GeneratedMerkleTreeCollection::new_from_file(&file)?
                    }
                };

                create_meta_merkle_tree(
                    cli.operator_address.clone(),
                    some_merkle_tree_collection,
                    epoch_to_process,
                    &cli.save_path,
                    // TODO: If we keep the separate thread for handling NCN submission
                    //  through files on disk then this needs to be true
                    save_stages,
                );
                stage = OperatorState::CastVote;
            }
            OperatorState::CastVote => {
                // TODO: Determine if this should be a stage given the task that's in a
                //  separate thread
            }
            OperatorState::WaitForNextEpoch => {
                current_epoch_info =
                    wait_for_next_epoch(&rpc_client, current_epoch_info.epoch).await;
                // Get the last slot of the previous epoch
                let (previous_epoch, previous_epoch_slot) =
                    if let Ok((epoch, slot)) = get_previous_epoch_last_slot(&rpc_client).await {
                        (epoch, slot)
                    } else {
                        // TODO: Make a datapoint error
                        error!("Error getting previous epoch slot");
                        continue;
                    };
                slot_to_process = previous_epoch_slot;
                epoch_to_process = previous_epoch;

                // TODO: When we start with wait for the next epoch, should we always wait
                //  for the optimal snapshot?
                let incremental_snapshots_path = cli.backup_snapshots_dir.clone();
                wait_for_optimal_incremental_snapshot(incremental_snapshots_path, slot_to_process)
                    .await?;
                stage = OperatorState::LoadBankFromSnapshot;
            }
        }
    }
}

use std::{
    path::PathBuf,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use ellipsis_client::EllipsisClient;
use log::{error, info};
use meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_runtime::bank::Bank;
use solana_sdk::{epoch_info::EpochInfo, pubkey::Pubkey, signature::read_keypair_file};
use tokio::time;

use crate::{
    backup_snapshots::SnapshotInfo, cli::SnapshotPaths, create_merkle_tree_collection,
    create_meta_merkle_tree, create_stake_meta, ledger_utils::get_bank_from_snapshot_at_slot,
    load_bank_from_snapshot, meta_merkle_tree_path, read_merkle_tree_collection,
    read_stake_meta_collection, submit::submit_to_ncn, tip_router::get_ncn_config, Cli,
    OperatorState, Version,
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
    let keypair = read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file");
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
                let incremental_snapshots_path = cli.backup_snapshots_dir.clone();
                wait_for_optimal_incremental_snapshot(incremental_snapshots_path, slot_to_process)
                    .await?;

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
                    let SnapshotPaths {
                        ledger_path,
                        account_paths,
                        full_snapshots_path: _,
                        incremental_snapshots_path: _,
                        backup_snapshots_dir,
                    } = cli.get_snapshot_paths();

                    // Ensure that the local RPC node is caught up to the slot we want to process
                    let try_catchup =
                        crate::solana_cli::catchup(cli.rpc_url.to_owned(), cli.localhost_port);
                    if let Err(e) = try_catchup {
                        datapoint_error!(
                            "tip_router_cli.create_stake_meta",
                            ("operator_address", operator_address, String),
                            ("epoch", epoch_to_process, i64),
                            ("status", "error", String),
                            ("error", e.to_string(), String),
                            ("state", "create_stake_meta", String),
                            ("duration_ms", start.elapsed().as_millis() as i64, i64),
                            "cluster" => &cli.cluster,
                        );
                        panic!("Failed to catch up: {}", e);
                    }

                    // We can safely expect to use the backup_snapshots_dir as the full snapshot path because
                    //  _get_bank_from_snapshot_at_slot_ expects the snapshot at the exact `slot` to have
                    //  already been taken.
                    let maybe_bank = get_bank_from_snapshot_at_slot(
                        slot_to_process,
                        &backup_snapshots_dir,
                        &backup_snapshots_dir,
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
                                ("duration_ms", start.elapsed().as_millis() as i64, i64),
                                "cluster" => &cli.cluster,
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
                    &cli.get_save_path(),
                    save_stages,
                    &cli.cluster,
                ));
                // we should be able to safely drop the bank in this loop
                bank = None;
                // Transition to the next stage
                stage = OperatorState::CreateMerkleTreeCollection;
            }
            OperatorState::CreateMerkleTreeCollection => {
                let some_stake_meta_collection = stake_meta_collection.to_owned().map_or_else(
                    || read_stake_meta_collection(epoch_to_process, &cli.get_save_path()),
                    |collection| collection,
                );
                let config =
                    get_ncn_config(&rpc_client, tip_router_program_id, ncn_address).await?;
                // Tip Router looks backwards in time (typically current_epoch - 1) to calculated
                //  distributions. Meanwhile the NCN's Ballot is for the current_epoch. So we
                //  use epoch + 1 here
                let ballot_epoch = epoch_to_process.checked_add(1).unwrap();
                let protocol_fee_bps = config.fee_config.adjusted_total_fees_bps(ballot_epoch)?;

                // Generate the merkle tree collection
                merkle_tree_collection = Some(create_merkle_tree_collection(
                    cli.operator_address.clone(),
                    tip_router_program_id,
                    some_stake_meta_collection,
                    epoch_to_process,
                    ncn_address,
                    protocol_fee_bps,
                    &cli.get_save_path(),
                    save_stages,
                    &cli.cluster,
                ));

                stake_meta_collection = None;
                // Transition to the next stage
                stage = OperatorState::CreateMetaMerkleTree;
            }
            OperatorState::CreateMetaMerkleTree => {
                let some_merkle_tree_collection = merkle_tree_collection.to_owned().map_or_else(
                    || read_merkle_tree_collection(epoch_to_process, &cli.get_save_path()),
                    |collection| collection,
                );

                let merkle_tree = create_meta_merkle_tree(
                    cli.operator_address.clone(),
                    some_merkle_tree_collection,
                    epoch_to_process,
                    &cli.get_save_path(),
                    // This is defaulted to true because the output file is required by the
                    //  task that sets TipDistributionAccounts' merkle roots
                    true,
                    &cli.cluster,
                );
                datapoint_info!(
                    "tip_router_cli.process_epoch",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch_to_process, i64),
                    ("status", "success", String),
                    ("state", "epoch_processing_completed", String),
                    (
                        "meta_merkle_root",
                        format!("{:?}", merkle_tree.merkle_root),
                        String
                    ),
                    ("version", Version::default().to_string(), String),
                    "cluster" => &cli.cluster,
                );
                stage = OperatorState::CastVote;
            }
            OperatorState::CastVote => {
                let meta_merkle_tree_path =
                    meta_merkle_tree_path(epoch_to_process, &cli.get_save_path());

                let operator_address = Pubkey::from_str(&cli.operator_address)?;
                submit_to_ncn(
                    &rpc_client,
                    &keypair,
                    &operator_address,
                    &meta_merkle_tree_path,
                    epoch_to_process,
                    ncn_address,
                    tip_router_program_id,
                    tip_distribution_program_id,
                    cli.submit_as_memo,
                    // We let the submit task handle setting merkle roots
                    false,
                    &cli.cluster,
                )
                .await?;
                stage = OperatorState::WaitForNextEpoch;
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

                stage = OperatorState::LoadBankFromSnapshot;
            }
        }
    }
}

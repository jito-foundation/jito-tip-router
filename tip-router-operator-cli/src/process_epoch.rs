use std::{str::FromStr, sync::Arc, time::Duration};

use crate::{
    create_merkle_tree_collection, create_meta_merkle_tree, meta_merkle_tree_path,
    read_merkle_tree_collection, read_stake_meta_collection, reclaim,
    stake_meta_watcher::wait_for_stake_meta, submit::submit_to_ncn, tip_router::get_ncn_config,
    Cli, OperatorState, Version,
};
use anyhow::Result;
use log::{error, info};
use meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{epoch_info::EpochInfo, pubkey::Pubkey, signature::Keypair};

pub async fn wait_for_next_epoch(rpc_client: &RpcClient, current_epoch: u64) -> EpochInfo {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await;
        let new_epoch_info = match rpc_client.get_epoch_info().await {
            Ok(info) => info,
            Err(error) => {
                error!("Error getting epoch info: {error:?}");
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

#[allow(clippy::too_many_arguments)]
pub async fn loop_stages(
    keypair: Arc<Keypair>,
    rpc_client: Arc<RpcClient>,
    cli: Cli,
    starting_stage: OperatorState,
    _override_target_slot: Option<u64>,
    tip_router_program_id: &Pubkey,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    _tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    _enable_snapshots: bool,
    save_stages: bool,
    reclaim_expired_accounts: bool,
    num_monitored_epochs: u64,
) -> Result<()> {
    let mut current_epoch_info = {
        loop {
            match rpc_client.get_epoch_info().await {
                Ok(info) => break info,
                Err(e) => {
                    error!("Error getting epoch info from RPC. Retrying...");
                    datapoint_error!(
                        "tip_router_cli.get_epoch_info",
                        ("operator_address", cli.operator_address.clone(), String),
                        ("status", "error", String),
                        ("error", e.to_string(), String),
                        "cluster" => &cli.cluster,
                    );
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
        }
    };

    let operator_address = cli.operator_address.clone();
    let mut stage = starting_stage;
    let mut stake_meta_collection: Option<StakeMetaCollection> = None;
    let mut merkle_tree_collection: Option<GeneratedMerkleTreeCollection> = None;
    let mut epoch_to_process = current_epoch_info.epoch.saturating_sub(1);
    loop {
        match stage {
            OperatorState::WatchForStakeMeta => {
                let stake_meta_directory = cli.get_save_path();
                let watched_epoch = epoch_to_process;
                info!(
                    "Waiting for stake-meta collection for epoch {watched_epoch} in {}",
                    stake_meta_directory.display()
                );
                stake_meta_collection = Some(
                    tokio::task::spawn_blocking(move || {
                        wait_for_stake_meta(stake_meta_directory, watched_epoch)
                    })
                    .await??,
                );
                stage = OperatorState::CreateMerkleTreeCollection;
            }
            OperatorState::CreateMerkleTreeCollection => {
                let config =
                    get_ncn_config(&rpc_client, tip_router_program_id, ncn_address).await?;
                // Tip Router looks backwards in time (typically current_epoch - 1) to calculated
                //  distributions. Meanwhile the NCN's Ballot is for the current_epoch. So we
                //  use epoch + 1 here
                let ballot_epoch = epoch_to_process
                    .checked_add(1)
                    .expect("ballot epoch should fit in u64");
                let fees = config.fee_config.current_fees(ballot_epoch);
                let protocol_fee_bps = config.fee_config.adjusted_total_fees_bps(ballot_epoch)?;

                // Generate the merkle tree collection
                let some_stake_meta_collection =
                    stake_meta_collection.to_owned().unwrap_or_else(|| {
                        read_stake_meta_collection(epoch_to_process, &cli.get_save_path())
                    });
                merkle_tree_collection = Some(create_merkle_tree_collection(
                    cli.operator_address.clone(),
                    tip_router_program_id,
                    some_stake_meta_collection,
                    epoch_to_process,
                    ncn_address,
                    protocol_fee_bps,
                    fees.priority_fee_distribution_fee_bps(),
                    &cli.get_save_path(),
                    save_stages,
                    &cli.cluster,
                ));

                stake_meta_collection = None;
                // Transition to the next stage
                stage = OperatorState::CreateMetaMerkleTree;
            }
            OperatorState::CreateMetaMerkleTree => {
                let merkle_root = {
                    let some_merkle_tree_collection =
                        merkle_tree_collection.to_owned().unwrap_or_else(|| {
                            read_merkle_tree_collection(epoch_to_process, &cli.get_save_path())
                        });
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
                    merkle_tree.merkle_root
                };

                datapoint_info!(
                    "tip_router_cli.process_epoch",
                    ("operator_address", operator_address, String),
                    ("epoch", epoch_to_process, i64),
                    ("status", "success", String),
                    ("state", "epoch_processing_completed", String),
                    (
                        "meta_merkle_root",
                        format!("{:?}", merkle_root),
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
                let submit_result = submit_to_ncn(
                    &rpc_client,
                    &keypair,
                    &operator_address,
                    &meta_merkle_tree_path,
                    epoch_to_process,
                    ncn_address,
                    tip_router_program_id,
                    tip_distribution_program_id,
                    priority_fee_distribution_program_id,
                    cli.submit_as_memo,
                    // We let the submit task handle setting merkle roots
                    false,
                    cli.vote_microlamports,
                    &cli.cluster,
                )
                .await;
                if let Err(e) = submit_result {
                    error!(
                        "Failed to submit epoch {} to NCN: {:?}",
                        epoch_to_process, e
                    );
                    datapoint_error!(
                        "tip_router_cli.cast_vote",
                        ("operator_address", operator_address.to_string(), String),
                        ("epoch", epoch_to_process, i64),
                        ("status", "error", String),
                        ("error", e.to_string(), String),
                        ("state", "cast_vote", String),
                        "cluster" => &cli.cluster,
                    );
                }
                stage = OperatorState::ReclaimExpiredAccounts;
            }
            OperatorState::ReclaimExpiredAccounts => {
                if reclaim_expired_accounts {
                    info!("Checking for expired accounts to close...");
                    if let Err(e) = reclaim::close_expired_accounts(
                        &cli.rpc_url,
                        *tip_distribution_program_id,
                        *priority_fee_distribution_program_id,
                        keypair.clone(),
                        num_monitored_epochs,
                    )
                    .await
                    {
                        error!("Error closing expired accounts: {e}");
                    }
                }

                stage = OperatorState::WaitForNextEpoch;
            }
            OperatorState::WaitForNextEpoch => {
                current_epoch_info =
                    wait_for_next_epoch(&rpc_client, current_epoch_info.epoch).await;
                epoch_to_process = current_epoch_info.epoch.saturating_sub(1);
                stage = OperatorState::WatchForStakeMeta;
            }
        }
    }
}

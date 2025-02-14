use std::{
    path::PathBuf,
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::Result;
use ellipsis_client::EllipsisClient;
use log::{error, info};
use solana_metrics::{datapoint_error, datapoint_info};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use tokio::time;

use crate::{
    backup_snapshots::SnapshotInfo, get_meta_merkle_root, tip_router::get_ncn_config, Cli,
};

const MAX_WAIT_FOR_INCREMENTAL_SNAPSHOT_TICKS: u64 = 1200; // Experimentally determined
const OPTIMAL_INCREMENTAL_SNAPSHOT_SLOT_RANGE: u64 = 800; // Experimentally determined

pub async fn wait_for_next_epoch(rpc_client: &RpcClient, current_epoch: u64) -> u64 {
    loop {
        tokio::time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        let new_epoch = match rpc_client.get_epoch_info().await {
            Ok(info) => info.epoch,
            Err(e) => {
                error!("Error getting epoch info: {:?}", e);
                continue;
            }
        };

        if new_epoch > current_epoch {
            info!("New epoch detected: {} -> {}", current_epoch, new_epoch);
            return new_epoch;
        }
    }
}

pub async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<(u64, u64)> {
    let epoch_info = rpc_client.get_epoch_info().await?;
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
pub async fn process_epoch(
    client: &EllipsisClient,
    target_slot: u64,
    target_epoch: u64,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    tip_router_program_id: &Pubkey,
    ncn_address: &Pubkey,
    snapshots_enabled: bool,
    new_epoch_rollover: bool,
    cli_args: &Cli,
) -> Result<()> {
    info!("Processing epoch {:?}", target_epoch);

    let start = Instant::now();

    let ledger_path = cli_args.ledger_path.clone();
    let account_paths = None;
    let full_snapshots_path = cli_args.full_snapshots_path.clone();
    let incremental_snapshots_path = cli_args.backup_snapshots_dir.clone();
    let operator_address = Pubkey::from_str(&cli_args.operator_address).unwrap();
    let meta_merkle_tree_dir = cli_args.meta_merkle_tree_dir.clone();

    // Get the protocol fees
    let ncn_config = get_ncn_config(client, tip_router_program_id, ncn_address).await?;
    let tip_router_target_epoch = target_epoch
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("tip_router_target_epoch overflow"))?;
    let adjusted_total_fees = ncn_config
        .fee_config
        .adjusted_total_fees_bps(tip_router_target_epoch)?;

    let account_paths = account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
    let full_snapshots_path = full_snapshots_path.map_or(ledger_path, |path| path);

    // Wait for optimal incremental snapshot to be available since they can be delayed in a new epoch
    if new_epoch_rollover {
        wait_for_optimal_incremental_snapshot(incremental_snapshots_path.clone(), target_slot)
            .await?;
    }

    // Generate merkle root from ledger
    let meta_merkle_tree = match get_meta_merkle_root(
        cli_args.ledger_path.as_path(),
        account_paths,
        full_snapshots_path,
        incremental_snapshots_path,
        &target_slot,
        tip_distribution_program_id,
        "", // TODO out_path is not used, unsure what should be put here. Maybe `snapshot_output_dir` from cli args?
        tip_payment_program_id,
        tip_router_program_id,
        ncn_address,
        &operator_address,
        target_epoch,
        adjusted_total_fees,
        snapshots_enabled,
        &meta_merkle_tree_dir,
    ) {
        Ok(tree) => {
            datapoint_info!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address.to_string(), String),
                ("epoch", target_epoch, i64),
                ("status", "success", String),
                ("state", "merkle_root_generation", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
            tree
        }
        Err(e) => {
            datapoint_error!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address.to_string(), String),
                ("epoch", target_epoch, i64),
                ("status", "error", String),
                ("error", format!("{:?}", e), String),
                ("state", "merkle_root_generation", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
            return Err(anyhow::anyhow!("Failed to generate merkle root: {:?}", e));
        }
    };

    // Write meta merkle tree to file
    let meta_merkle_tree_path =
        meta_merkle_tree_dir.join(format!("meta_merkle_tree_{}.json", target_epoch));
    let meta_merkle_tree_json = match serde_json::to_string(&meta_merkle_tree) {
        Ok(json) => json,
        Err(e) => {
            datapoint_error!(
                "tip_router_cli.process_epoch",
                ("operator_address", operator_address.to_string(), String),
                ("epoch", target_epoch, i64),
                ("status", "error", String),
                ("error", format!("{:?}", e), String),
                ("state", "merkle_root_serialization", String),
                ("duration_ms", start.elapsed().as_millis() as i64, i64)
            );
            return Err(anyhow::anyhow!(
                "Failed to serialize meta merkle tree: {}",
                e
            ));
        }
    };

    if let Err(e) = std::fs::write(meta_merkle_tree_path, meta_merkle_tree_json) {
        datapoint_error!(
            "tip_router_cli.process_epoch",
            ("operator_address", operator_address.to_string(), String),
            ("epoch", target_epoch, i64),
            ("status", "error", String),
            ("error", format!("{:?}", e), String),
            ("state", "merkle_root_file_write", String),
            ("duration_ms", start.elapsed().as_millis() as i64, i64)
        );
        return Err(anyhow::anyhow!(
            "Failed to write meta merkle tree to file: {}",
            e
        ));
    }

    // Emit a datapoint for starting the epoch processing
    datapoint_info!(
        "tip_router_cli.process_epoch",
        ("operator_address", operator_address.to_string(), String),
        ("epoch", target_epoch, i64),
        ("status", "success", String),
        ("state", "epoch_processing_completed", String),
        (
            "meta_merkle_root",
            format!("{:?}", meta_merkle_tree.merkle_root),
            String
        ),
        ("duration_ms", start.elapsed().as_millis() as i64, i64)
    );

    solana_metrics::flush();

    Ok(())
}

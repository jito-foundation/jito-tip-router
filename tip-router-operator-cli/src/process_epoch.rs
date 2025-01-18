use std::{
    str::FromStr,
    time::{Duration, Instant},
};

use anyhow::Result;
use ellipsis_client::EllipsisClient;
use log::info;
use solana_metrics::{datapoint_error, datapoint_info};
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair};

use crate::{
    get_meta_merkle_root,
    tip_router::{cast_vote, get_ncn_config},
    Cli,
};

pub async fn wait_for_next_epoch(rpc_client: &RpcClient) -> Result<()> {
    let current_epoch = rpc_client.get_epoch_info()?.epoch;

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        let new_epoch = rpc_client.get_epoch_info()?.epoch;

        if new_epoch > current_epoch {
            info!("New epoch detected: {} -> {}", current_epoch, new_epoch);
            return Ok(());
        }
    }
}

pub async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<(u64, u64)> {
    let epoch_info = rpc_client.get_epoch_info()?;
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
pub async fn process_epoch(
    client: &EllipsisClient,
    target_slot: u64,
    target_epoch: u64,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    tip_router_program_id: &Pubkey,
    snapshots_enabled: bool,
    cli_args: &Cli,
) -> Result<()> {
    info!("Processing epoch {:?}", target_epoch);

    let start = Instant::now();

    let ledger_path = cli_args.ledger_path.clone();
    let account_paths = cli_args.account_paths.clone();
    let full_snapshots_path = cli_args.full_snapshots_path.clone();
    let operator_address = Pubkey::from_str(&cli_args.operator_address).unwrap();
    let meta_merkle_tree_dir = cli_args.meta_merkle_tree_dir.clone();

    // Get the protocol fees
    let ncn_config = get_ncn_config(client, tip_router_program_id, ncn_address).await?;
    let adjusted_total_fees = ncn_config
        .fee_config
        .adjusted_total_fees_bps(target_epoch)?;

    let account_paths = account_paths.map_or_else(|| vec![ledger_path.clone()], |paths| paths);
    let full_snapshots_path = full_snapshots_path.map_or(ledger_path, |path| path);

    // Generate merkle root from ledger
    let meta_merkle_tree = match get_meta_merkle_root(
        cli_args.ledger_path.as_path(),
        account_paths,
        full_snapshots_path,
        &target_slot,
        tip_distribution_program_id,
        "", // TODO out_path is not used, unsure what should be put here. Maybe `snapshot_output_dir` from cli args?
        tip_payment_program_id,
        ncn_address,
        &operator_address,
        target_epoch,
        adjusted_total_fees,
        snapshots_enabled,
    ) {
        Ok(tree) => {
            datapoint_info!(
                "tip_router_cli-process_epoch_success",
                ("epoch", target_epoch, i64)
            );
            tree
        }
        Err(e) => {
            datapoint_error!(
                "tip_router_cli-process_epoch_error",
                ("epoch", target_epoch, i64),
                ("error", format!("{:?}", e), String)
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
                "tip_router_cli-process_epoch_error",
                ("epoch", target_epoch, i64),
                ("error", format!("{:?}", e), String)
            );
            return Err(anyhow::anyhow!(
                "Failed to serialize meta merkle tree: {}",
                e
            ));
        }
    };

    if let Err(e) = std::fs::write(&meta_merkle_tree_path, meta_merkle_tree_json) {
        datapoint_error!(
            "tip_router_cli-process_epoch_error",
            ("epoch", target_epoch, i64),
            ("error", format!("{:?}", e), String)
        );
        return Err(anyhow::anyhow!(
            "Failed to write meta merkle tree to file: {}",
            e
        ));
    }

    let elapsed_us = start.elapsed().as_micros();
    // Emit a datapoint for starting the epoch processing
    datapoint_info!(
        "tip_router_cli-process_epoch_success",
        ("epoch", target_epoch, i64),
        ("elapsed_us", elapsed_us, i64),
    );

    solana_metrics::flush();

    Ok(())
}

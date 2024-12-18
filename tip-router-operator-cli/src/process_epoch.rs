use std::{str::FromStr, time::Duration};

use anyhow::Result;
use ellipsis_client::EllipsisClient;
use log::info;
use solana_rpc_client::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, signer::keypair::Keypair};

use crate::{
    get_merkle_root,
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

    let epoch_start_slot = current_slot - slot_index;
    let previous_epoch_final_slot = epoch_start_slot.saturating_sub(1);
    let previous_epoch = epoch_info.epoch.saturating_sub(1);

    Ok((previous_epoch, previous_epoch_final_slot))
}

pub async fn process_epoch(
    client: &EllipsisClient,
    previous_epoch_slot: u64,
    previous_epoch: u64,
    payer: &Keypair,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey,
    cli_args: &Cli,
) -> Result<()> {
    info!("Processing epoch {:?}", previous_epoch);

    // TODO maybe better to pass these as args rather than the cli_args reference?
    let ledger_path = cli_args.ledger_path.clone();
    let account_paths = cli_args.account_paths.clone();
    let full_snapshots_path = cli_args.full_snapshots_path.clone();
    let operator = Pubkey::from_str(&cli_args.operator_address).unwrap();

    // Get the protocol fees
    let ncn_config = get_ncn_config(client, ncn_address).await.unwrap();
    let fees = ncn_config
        .fee_config
        .total_fees_bps(previous_epoch)
        .unwrap();

    let account_paths = match account_paths {
        Some(paths) => paths,
        None => vec![ledger_path.clone()],
    };
    let full_snapshots_path = match full_snapshots_path {
        Some(path) => path,
        None => ledger_path,
    };
    // Generate merkle root from ledger
    let meta_merkle_tree = get_merkle_root(
        cli_args.ledger_path.as_path(),
        account_paths,
        full_snapshots_path,
        &previous_epoch_slot,
        tip_distribution_program_id,
        "", // TODO out_path is not used, unsure what should be put here. Maybe `snapshot_output_dir` from cli args?
        tip_payment_program_id,
        fees,
    )
    .unwrap();

    // cast vote using the generated merkle root
    let tx_sig = cast_vote(
        client,
        payer,
        *ncn_address,
        operator,
        payer,
        meta_merkle_tree.merkle_root,
        // TODO determine if this is current or prev epoch. Where prev epoch
        // is the epoch in which StakeMetaCollection was created (i.e. epoch
        // corresponding to the previous_epoch_slot).
        previous_epoch,
    )
    .await
    .unwrap();

    info!("Successfully cast vote at tx {:?}", tx_sig);

    Ok(())
}

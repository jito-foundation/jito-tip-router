#![allow(clippy::integer_division)]
use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::EllipsisClient,
    log::{error, info},
    solana_metrics::set_host_id,
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        clock::DEFAULT_SLOTS_PER_EPOCH, pubkey::Pubkey, signer::keypair::read_keypair_file,
    },
    std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
        claim::claim_mev_tips_with_emit,
        cli::{Cli, Commands},
        process_epoch::{get_previous_epoch_last_slot, process_epoch, wait_for_next_epoch},
        submit::{submit_recent_epochs_to_ncn, submit_to_ncn},
    },
    tokio::time::sleep,
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let keypair = read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file");
    let rpc_client = EllipsisClient::from_rpc_with_timeout(
        RpcClient::new(cli.rpc_url.clone()),
        &read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
        1_800_000, // 30 minutes
    )?;

    set_host_id(cli.operator_address.to_string());

    // Ensure tx submission works
    // let test_meta_merkle_root = [1; 32];
    // let ix = spl_memo::build_memo(&test_meta_merkle_root.to_vec(), &[&keypair.pubkey()]);
    // info!("Submitting test tx {:?}", test_meta_merkle_root);
    // let tx = Transaction::new_with_payer(&[ix], Some(&keypair.pubkey()));
    // rpc_client.process_transaction(tx, &[&keypair]).await?;

    info!(
        "CLI Arguments:
        keypair_path: {}
        operator_address: {}
        rpc_url: {}
        ledger_path: {}
        full_snapshots_path: {:?}
        snapshot_output_dir: {}
        backup_snapshots_dir: {}",
        cli.keypair_path,
        cli.operator_address,
        cli.rpc_url,
        cli.ledger_path.display(),
        cli.full_snapshots_path,
        cli.snapshot_output_dir.display(),
        cli.backup_snapshots_dir.display()
    );

    match cli.command {
        Commands::Run {
            ncn_address,
            tip_distribution_program_id,
            tip_payment_program_id,
            tip_router_program_id,
            enable_snapshots,
            num_monitored_epochs,
            start_next_epoch,
            override_target_slot,
            set_merkle_roots,
            claim_tips,
        } => {
            info!("Running Tip Router...");
            info!("NCN Address: {}", ncn_address);
            info!(
                "Tip Distribution Program ID: {}",
                tip_distribution_program_id
            );
            info!("Tip Payment Program ID: {}", tip_payment_program_id);
            info!("Tip Router Program ID: {}", tip_router_program_id);
            info!("Enable Snapshots: {}", enable_snapshots);
            info!("Num Monitored Epochs: {}", num_monitored_epochs);
            info!("Start Next Epoch: {}", start_next_epoch);
            info!("Override Target Slot: {:?}", override_target_slot);
            info!("Submit as Memo: {}", cli.submit_as_memo);

            let rpc_client_clone = rpc_client.clone();
            let full_snapshots_path = cli.full_snapshots_path.clone().unwrap();
            let backup_snapshots_dir = cli.backup_snapshots_dir.clone();
            let rpc_url = cli.rpc_url.clone();
            let cli_clone = cli.clone();
            let mut current_epoch = rpc_client.get_epoch_info()?.epoch;

            if !backup_snapshots_dir.exists() {
                info!(
                    "Creating backup snapshots directory at {}",
                    backup_snapshots_dir.display()
                );
                std::fs::create_dir_all(&backup_snapshots_dir)?;
            }

            // Check for new meta merkle trees and submit to NCN periodically
            tokio::spawn(async move {
                let keypair_arc = Arc::new(keypair);
                loop {
                    if let Err(e) = submit_recent_epochs_to_ncn(
                        &rpc_client_clone,
                        &keypair_arc,
                        &ncn_address,
                        &tip_router_program_id,
                        &tip_distribution_program_id,
                        num_monitored_epochs,
                        &cli_clone,
                        set_merkle_roots,
                    )
                    .await
                    {
                        error!("Error submitting to NCN: {}", e);
                    }
                    sleep(Duration::from_secs(600)).await;
                }
            });

            // Track incremental snapshots and backup to `backup_snapshots_dir`
            tokio::spawn(async move {
                loop {
                    if let Err(e) = BackupSnapshotMonitor::new(
                        &rpc_url,
                        full_snapshots_path.clone(),
                        backup_snapshots_dir.clone(),
                        override_target_slot,
                    )
                    .run()
                    .await
                    {
                        error!("Error running backup snapshot monitor: {}", e);
                    }
                }
            });

            // Run claims if enabled
            if claim_tips {
                let cli_clone = cli.clone();
                let rpc_client = rpc_client.clone();
                tokio::spawn(async move {
                    loop {
                        // Slow process with lots of account fetches so run every 30 minutes
                        sleep(Duration::from_secs(1800)).await;
                        let epoch = if let Ok(epoch) = rpc_client.get_epoch_info() {
                            epoch.epoch.checked_sub(1).unwrap_or(epoch.epoch)
                        } else {
                            continue;
                        };
                        if let Err(e) = claim_mev_tips_with_emit(
                            &cli_clone,
                            epoch,
                            tip_distribution_program_id,
                            tip_router_program_id,
                            ncn_address,
                            Duration::from_secs(3600),
                        )
                        .await
                        {
                            error!("Error claiming tips: {}", e);
                        }
                    }
                });
            }

            if start_next_epoch {
                current_epoch = wait_for_next_epoch(&rpc_client, current_epoch).await;
            }

            // Track runs that are starting right at the beginning of a new epoch
            let mut new_epoch_rollover = start_next_epoch;

            loop {
                // Get the last slot of the previous epoch
                let (previous_epoch, previous_epoch_slot) =
                    if let Ok((epoch, slot)) = get_previous_epoch_last_slot(&rpc_client) {
                        (epoch, slot)
                    } else {
                        error!("Error getting previous epoch slot");
                        continue;
                    };

                info!("Processing slot {} for previous epoch", previous_epoch_slot);

                // Process the epoch
                match process_epoch(
                    &rpc_client,
                    previous_epoch_slot,
                    previous_epoch,
                    &tip_distribution_program_id,
                    &tip_payment_program_id,
                    &tip_router_program_id,
                    &ncn_address,
                    enable_snapshots,
                    new_epoch_rollover,
                    &cli,
                )
                .await
                {
                    Ok(_) => info!("Successfully processed epoch"),
                    Err(e) => {
                        error!("Error processing epoch: {}", e);
                    }
                }

                // Wait for epoch change
                current_epoch = wait_for_next_epoch(&rpc_client, current_epoch).await;

                new_epoch_rollover = true;
            }
        }
        Commands::SnapshotSlot {
            ncn_address,
            tip_distribution_program_id,
            tip_payment_program_id,
            tip_router_program_id,
            enable_snapshots,
            slot,
        } => {
            info!("Snapshotting slot...");
            let epoch = slot / DEFAULT_SLOTS_PER_EPOCH;
            // Process the epoch
            match process_epoch(
                &rpc_client,
                slot,
                epoch,
                &tip_distribution_program_id,
                &tip_payment_program_id,
                &tip_router_program_id,
                &ncn_address,
                enable_snapshots,
                false,
                &cli,
            )
            .await
            {
                Ok(_) => info!("Successfully processed slot"),
                Err(e) => {
                    error!("Error processing epoch: {}", e);
                }
            }
        }
        Commands::SubmitEpoch {
            ncn_address,
            tip_distribution_program_id,
            tip_router_program_id,
            epoch,
        } => {
            let meta_merkle_tree_path = PathBuf::from(format!(
                "{}/meta_merkle_tree_{}.json",
                cli.meta_merkle_tree_dir.display(),
                epoch
            ));
            info!(
                "Submitting epoch {} from {}...",
                epoch,
                meta_merkle_tree_path.display()
            );
            let operator_address = Pubkey::from_str(&cli.operator_address)?;
            submit_to_ncn(
                &rpc_client,
                &keypair,
                &operator_address,
                &meta_merkle_tree_path,
                epoch,
                &ncn_address,
                &tip_router_program_id,
                &tip_distribution_program_id,
                cli.submit_as_memo,
                true,
            )
            .await?;
        }
        Commands::ClaimTips {
            tip_distribution_program_id,
            tip_router_program_id,
            ncn_address,
            epoch,
        } => {
            info!("Claiming tips...");

            claim_mev_tips_with_emit(
                &cli,
                epoch,
                tip_distribution_program_id,
                tip_router_program_id,
                ncn_address,
                Duration::from_secs(3600),
            )
            .await?;
        }
    }
    Ok(())
}

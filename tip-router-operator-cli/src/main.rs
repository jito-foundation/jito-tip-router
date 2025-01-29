use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::{ClientSubset, EllipsisClient},
    log::{error, info},
    solana_metrics::{datapoint_info, set_host_id},
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        clock::DEFAULT_SLOTS_PER_EPOCH,
        pubkey::Pubkey,
        signer::{keypair::read_keypair_file, Signer},
        transaction::Transaction,
    },
    std::{path::PathBuf, str::FromStr, time::Duration},
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
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
    let rpc_client = EllipsisClient::from_rpc(
        RpcClient::new(cli.rpc_url.clone()),
        &read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
    )?;

    set_host_id(cli.operator_address.to_string());

    let test_meta_merkle_root = [1; 32];
    let ix = spl_memo::build_memo(&test_meta_merkle_root.to_vec(), &[&keypair.pubkey()]);
    info!("Submitting test memo {:?}", test_meta_merkle_root);

    let tx = Transaction::new_with_payer(&[ix], Some(&keypair.pubkey()));
    rpc_client.process_transaction(tx, &[&keypair]).await?;

    info!("CLI Arguments: {:?}", cli);

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
        } => {
            info!("Running Tip Router...");

            let rpc_client_clone = rpc_client.clone();
            let full_snapshots_path = cli.full_snapshots_path.clone().unwrap();
            let backup_snapshots_dir = cli.backup_snapshots_dir.clone();
            let rpc_url = cli.rpc_url.clone();
            let cli_clone = cli.clone();

            if !backup_snapshots_dir.exists() {
                info!(
                    "Creating backup snapshots directory at {}",
                    backup_snapshots_dir.display()
                );
                std::fs::create_dir_all(&backup_snapshots_dir)?;
            }

            

            // Check for new meta merkle trees and submit to NCN periodically
            tokio::spawn(async move {
                loop {
                    if let Err(e) = submit_recent_epochs_to_ncn(
                        &rpc_client_clone,
                        &keypair,
                        &ncn_address,
                        &tip_router_program_id,
                        &tip_distribution_program_id,
                        num_monitored_epochs,
                        &cli_clone,
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

            if start_next_epoch {
                wait_for_next_epoch(&rpc_client).await?;
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
                if let Err(e) = wait_for_next_epoch(&rpc_client).await {
                    error!("Error waiting for next epoch: {}", e);
                    sleep(Duration::from_secs(60)).await;
                }

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
            )
            .await?;
        }
    }
    Ok(())
}

#![allow(clippy::integer_division)]
use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::EllipsisClient,
    log::{error, info},
    meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection},
    solana_metrics::set_host_id,
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{pubkey::Pubkey, signer::keypair::read_keypair_file},
    std::{str::FromStr, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
        claim::claim_mev_tips_with_emit,
        cli::{Cli, Commands, SnapshotPaths},
        create_merkle_tree_collection, create_meta_merkle_tree, create_stake_meta,
        ledger_utils::get_bank_from_snapshot_at_slot,
        load_bank_from_snapshot, merkle_tree_collection_file_name, meta_merkle_tree_file_name,
        process_epoch, stake_meta_file_name,
        submit::{submit_recent_epochs_to_ncn, submit_to_ncn},
        tip_router::get_ncn_config,
    },
    tokio::{sync::Mutex, time::sleep},
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    // Ensure backup directory and
    cli.force_different_backup_snapshot_dir();

    let keypair = read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file");
    let rpc_client = EllipsisClient::from_rpc_with_timeout(
        RpcClient::new(cli.rpc_url.clone()),
        &read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
        1_800_000, // 30 minutes
    )?;

    set_host_id(cli.operator_address.to_string());

    // Will panic if the user did not set --save-path or the deprecated --meta-merkle-tree-dir
    let save_path = cli.get_save_path();

    info!(
        "CLI Arguments:
        keypair_path: {}
        operator_address: {}
        rpc_url: {}
        ledger_path: {}
        full_snapshots_path: {:?}
        snapshot_output_dir: {}
        backup_snapshots_dir: {}
        save_path: {}",
        cli.keypair_path,
        cli.operator_address,
        cli.rpc_url,
        cli.ledger_path.display(),
        cli.full_snapshots_path,
        cli.snapshot_output_dir.display(),
        cli.backup_snapshots_dir.display(),
        save_path.display(),
    );

    cli.create_save_path();

    match cli.command {
        Commands::Run {
            ncn_address,
            tip_distribution_program_id,
            tip_payment_program_id,
            tip_router_program_id,
            save_snapshot,
            num_monitored_epochs,
            override_target_slot,
            starting_stage,
            save_stages,
            set_merkle_roots,
            claim_tips,
        } => {
            assert!(
                num_monitored_epochs > 0,
                "num-monitored-epochs must be greater than 0"
            );

            info!("Running Tip Router...");
            info!("NCN Address: {}", ncn_address);
            info!(
                "Tip Distribution Program ID: {}",
                tip_distribution_program_id
            );
            info!("Tip Payment Program ID: {}", tip_payment_program_id);
            info!("Tip Router Program ID: {}", tip_router_program_id);
            info!("Save Snapshots: {}", save_snapshot);
            info!("Num Monitored Epochs: {}", num_monitored_epochs);
            info!("Override Target Slot: {:?}", override_target_slot);
            info!("Submit as Memo: {}", cli.submit_as_memo);
            info!("starting stage: {:?}", starting_stage);

            let rpc_client_clone = rpc_client.clone();
            let full_snapshots_path = cli.full_snapshots_path.clone().unwrap();
            let backup_snapshots_dir = cli.backup_snapshots_dir.clone();
            let rpc_url = cli.rpc_url.clone();
            let cli_clone: Cli = cli.clone();

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

            let cli_clone: Cli = cli.clone();
            // Track incremental snapshots and backup to `backup_snapshots_dir`
            tokio::spawn(async move {
                let save_path = cli_clone.get_save_path();
                loop {
                    if let Err(e) = BackupSnapshotMonitor::new(
                        &rpc_url,
                        full_snapshots_path.clone(),
                        backup_snapshots_dir.clone(),
                        override_target_slot,
                        save_path.clone(),
                        num_monitored_epochs,
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
                let file_mutex = Arc::new(Mutex::new(()));

                tokio::spawn(async move {
                    loop {
                        // Get current epoch
                        let current_epoch = match rpc_client.get_epoch_info().await {
                            Ok(epoch_info) => epoch_info.epoch,
                            Err(_) => {
                                // If we can't get the epoch, wait and retry
                                sleep(Duration::from_secs(60)).await;
                                continue;
                            }
                        };

                        // Create a vector to hold all our handles
                        let mut join_handles = Vec::new();

                        // Process current epoch and the previous two epochs
                        for epoch_offset in 0..3 {
                            // Make sure we don't go below epoch 0
                            if epoch_offset > current_epoch {
                                continue;
                            }

                            let epoch_to_process = current_epoch - epoch_offset;
                            let cli_ref = cli_clone.clone();
                            let file_mutex_ref = file_mutex.clone();

                            // Create a task for each epoch and add its handle to our vector
                            let handle = tokio::spawn(async move {
                                let result = claim_mev_tips_with_emit(
                                    &cli_ref,
                                    epoch_to_process,
                                    tip_distribution_program_id,
                                    tip_router_program_id,
                                    ncn_address,
                                    Duration::from_secs(3600),
                                    &file_mutex_ref,
                                )
                                .await;

                                if let Err(e) = result {
                                    error!(
                                        "Error claiming tips for epoch {}: {}",
                                        epoch_to_process, e
                                    );
                                } else {
                                    info!(
                                        "Successfully processed claims for epoch {}",
                                        epoch_to_process
                                    );
                                }

                                epoch_to_process
                            });

                            join_handles.push(handle);
                        }

                        // Wait for all tasks to complete
                        let mut completed_epochs = Vec::new();
                        for handle in join_handles {
                            if let Ok(epoch) = handle.await {
                                completed_epochs.push(epoch);
                            }
                        }

                        info!(
                            "Completed processing claims for epochs: {:?}",
                            completed_epochs
                        );

                        // Sleep before the next iteration
                        info!("Sleeping for 30 minutes before next claim cycle");
                        sleep(Duration::from_secs(1800)).await;
                    }
                });
            }

            // Endless loop that transitions between stages of the operator process.
            process_epoch::loop_stages(
                rpc_client,
                cli,
                starting_stage,
                override_target_slot,
                &tip_router_program_id,
                &tip_distribution_program_id,
                &tip_payment_program_id,
                &ncn_address,
                save_snapshot,
                save_stages,
            )
            .await?;
        }
        Commands::SnapshotSlot { slot } => {
            info!("Snapshotting slot...");

            load_bank_from_snapshot(cli, slot, true);
        }
        Commands::SubmitEpoch {
            ncn_address,
            tip_distribution_program_id,
            tip_router_program_id,
            epoch,
            set_merkle_roots,
        } => {
            let meta_merkle_tree_path = cli.get_save_path().join(meta_merkle_tree_file_name(epoch));
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
                set_merkle_roots,
            )
            .await?;
        }
        Commands::ClaimTips {
            tip_router_program_id,
            tip_distribution_program_id,
            ncn_address,
            epoch,
        } => {
            info!("Claiming tips...");
            let file_mutex = Arc::new(Mutex::new(()));
            claim_mev_tips_with_emit(
                &cli,
                epoch,
                tip_distribution_program_id,
                tip_router_program_id,
                ncn_address,
                Duration::from_secs(3600),
                &file_mutex,
            )
            .await?;
        }
        Commands::CreateStakeMeta {
            epoch,
            slot,
            tip_distribution_program_id,
            tip_payment_program_id,
            save,
        } => {
            let SnapshotPaths {
                ledger_path,
                account_paths,
                full_snapshots_path: _,
                incremental_snapshots_path: _,
                backup_snapshots_dir,
            } = cli.get_snapshot_paths();

            // We can safely expect to use the backup_snapshots_dir as the full snapshot path because
            //  _get_bank_from_snapshot_at_slot_ expects the snapshot at the exact `slot` to have
            //  already been taken.
            let bank = get_bank_from_snapshot_at_slot(
                slot,
                &backup_snapshots_dir,
                &backup_snapshots_dir,
                account_paths,
                ledger_path.as_path(),
            )?;

            create_stake_meta(
                cli.operator_address,
                epoch,
                &Arc::new(bank),
                &tip_distribution_program_id,
                &tip_payment_program_id,
                &save_path,
                save,
            );
        }
        Commands::CreateMerkleTreeCollection {
            tip_router_program_id,
            ncn_address,
            epoch,
            save,
        } => {
            // Load the stake_meta_collection from disk
            let stake_meta_collection = match StakeMetaCollection::new_from_file(
                &cli.get_save_path().join(stake_meta_file_name(epoch)),
            ) {
                Ok(stake_meta_collection) => stake_meta_collection,
                Err(e) => panic!("{}", e),
            };
            let config = get_ncn_config(&rpc_client, &tip_router_program_id, &ncn_address).await?;
            // Tip Router looks backwards in time (typically current_epoch - 1) to calculated
            //  distributions. Meanwhile the NCN's Ballot is for the current_epoch. So we
            //  use epoch + 1 here
            let ballot_epoch = epoch.checked_add(1).unwrap();
            let protocol_fee_bps = config.fee_config.adjusted_total_fees_bps(ballot_epoch)?;

            // Generate the merkle tree collection
            create_merkle_tree_collection(
                cli.operator_address,
                &tip_router_program_id,
                stake_meta_collection,
                epoch,
                &ncn_address,
                protocol_fee_bps,
                &save_path,
                save,
            );
        }
        Commands::CreateMetaMerkleTree { epoch, save } => {
            // Load the stake_meta_collection from disk
            let merkle_tree_collection = match GeneratedMerkleTreeCollection::new_from_file(
                &save_path.join(merkle_tree_collection_file_name(epoch)),
            ) {
                Ok(merkle_tree_collection) => merkle_tree_collection,
                Err(e) => panic!("{}", e),
            };

            create_meta_merkle_tree(
                cli.operator_address,
                merkle_tree_collection,
                epoch,
                &save_path,
                save,
            );
        }
    }
    Ok(())
}

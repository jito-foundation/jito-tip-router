#![allow(clippy::integer_division)]
use ::{
    anyhow::Result,
    base64::{engine::general_purpose, Engine},
    clap::Parser,
    jito_bytemuck::{AccountDeserialize, Discriminator},
    jito_restaking_client::instructions::{
        InitializeOperatorVaultTicketBuilder, OperatorWarmupNcnBuilder,
    },
    jito_restaking_core::{
        config::Config as RestakingConfig, ncn_operator_state::NcnOperatorState,
        ncn_vault_ticket::NcnVaultTicket, operator_vault_ticket::OperatorVaultTicket,
    },
    log::{error, info, warn},
    solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig},
    solana_client::{
        rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
        rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
    },
    solana_metrics::{datapoint_error, datapoint_info, set_host_id},
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{
        pubkey::Pubkey,
        signer::{keypair::read_keypair_file, Signer},
        transaction::Transaction,
    },
    std::{process::Command, str::FromStr, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
        claim::{claim_mev_tips_with_emit, emit_claim_mev_tips_metrics},
        cli::{Cli, Commands, SnapshotPaths},
        create_merkle_tree_collection, create_meta_merkle_tree, create_stake_meta,
        ledger_utils::get_bank_from_snapshot_at_slot,
        load_bank_from_snapshot, merkle_tree_collection_file_name, meta_merkle_tree_path,
        process_epoch, read_merkle_tree_collection, read_stake_meta_collection,
        stake_meta_file_name,
        submit::{submit_recent_epochs_to_ncn, submit_to_ncn},
        tip_router::get_ncn_config,
        Version,
    },
    tokio::{sync::Mutex, time::sleep},
};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    let hostname_cmd = Command::new("hostname")
        .output()
        .expect("Failed to execute hostname command");

    let hostname = String::from_utf8_lossy(&hostname_cmd.stdout)
        .trim()
        .to_string();

    let host_id = format!(
        "tip-router-operator-{}-{}-{}",
        &cli.cluster, &cli.region, &hostname
    );

    set_host_id(host_id.clone());

    info!("Ensuring localhost RPC is caught up with remote validator...");

    // Ensure backup directory and
    cli.force_different_backup_snapshot_dir();

    let keypair = read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file");
    let keypair = Arc::new(keypair);
    let rpc_client = Arc::new(RpcClient::new(cli.rpc_url.clone()));

    datapoint_info!(
        "tip_router_cli.version",
        ("operator_address", cli.operator_address.to_string(), String),
        ("version", Version::default().to_string(), String),
        "cluster" => &cli.cluster,
    );

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
        save_path: {},
        vote_microlamports: {}
        claim_microlamports: {}",
        cli.keypair_path,
        cli.operator_address,
        cli.rpc_url,
        cli.ledger_path.display(),
        cli.full_snapshots_path,
        cli.snapshot_output_dir.display(),
        cli.backup_snapshots_dir.display(),
        save_path.display(),
        &cli.vote_microlamports,
        &cli.claim_microlamports,
    );

    cli.create_save_path();

    match cli.command {
        Commands::Run {
            ncn_address,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            tip_payment_program_id,
            tip_router_program_id,
            restaking_program_id,
            save_snapshot,
            num_monitored_epochs,
            override_target_slot,
            starting_stage,
            save_stages,
            set_merkle_roots,
            claim_tips,
            claim_tips_metrics,
            claim_tips_epoch_lookback,
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
            let claim_tips_epoch_filepath = cli.claim_tips_epoch_filepath.clone();
            let cli_clone: Cli = cli.clone();
            let operator_address = cli.operator_address.clone();
            let cluster = cli.cluster.clone();

            let restaking_program_id = restaking_program_id
                .map_or_else(|| jito_restaking_program::id(), |program_id| program_id);

            let slot = rpc_client.get_slot().await?;
            let restaking_config_addr =
                RestakingConfig::find_program_address(&restaking_program_id).0;
            let restaking_config_acc = rpc_client.get_account(&restaking_config_addr).await?;
            let restaking_config =
                RestakingConfig::try_from_slice_unchecked(&restaking_config_acc.data)?;

            let ncn_operator_state_addr = NcnOperatorState::find_program_address(
                &restaking_program_id,
                &ncn_address,
                &Pubkey::from_str(operator_address.as_str()).unwrap(),
            )
            .0;
            match rpc_client.get_account(&ncn_operator_state_addr).await {
                Ok(account) => {
                    let ncn_operator_state =
                        NcnOperatorState::try_from_slice_unchecked(&account.data)?;
                    if ncn_operator_state
                        .operator_opt_in_state
                        .is_active(slot, restaking_config.epoch_length())?
                    {
                        let mut ix_builder = OperatorWarmupNcnBuilder::new();
                        ix_builder
                            .config(restaking_config_addr)
                            .ncn(ncn_address)
                            .operator(Pubkey::from_str(operator_address.as_str()).unwrap())
                            .ncn_operator_state(ncn_operator_state_addr)
                            .admin(keypair.pubkey());
                        let mut ix = ix_builder.instruction();
                        ix.program_id = restaking_program_id;

                        let blockhash = rpc_client.get_latest_blockhash().await?;
                        let tx = Transaction::new_signed_with_payer(
                            &[ix],
                            Some(&keypair.pubkey()),
                            &[keypair.clone()],
                            blockhash,
                        );
                        let result = rpc_client.send_and_confirm_transaction(&tx).await?;

                        info!("Transaction confirmed: {:?}", result);
                    }
                }
                Err(e) => warn!("Failed to find NcnOperatorState, Please contact NCN admin!: {e}"),
            }

            let config = {
                let data_size = std::mem::size_of::<NcnVaultTicket>()
                    .checked_add(8)
                    .ok_or_else(|| anyhow::anyhow!("Failed to add"))?
                    .checked_add(32)
                    .ok_or_else(|| anyhow::anyhow!("Failed to add"))?;
                let mut slice = Vec::new();
                slice.extend(vec![NcnVaultTicket::DISCRIMINATOR, 0, 0, 0, 0, 0, 0, 0]);
                slice.extend_from_slice(ncn_address.as_array());
                let encoded_slice = general_purpose::STANDARD.encode(slice);
                let memcmp = RpcFilterType::Memcmp(Memcmp::new(
                    0,
                    MemcmpEncodedBytes::Base64(encoded_slice),
                ));
                RpcProgramAccountsConfig {
                    filters: Some(vec![RpcFilterType::DataSize(data_size as u64), memcmp]),
                    account_config: RpcAccountInfoConfig {
                        encoding: Some(UiAccountEncoding::Base64),
                        data_slice: Some(UiDataSliceConfig {
                            offset: 0,
                            length: data_size,
                        }),
                        commitment: None,
                        min_context_slot: None,
                    },
                    with_context: Some(false),
                    sort_results: Some(false),
                }
            };
            let ncn_vault_tickets = rpc_client
                .get_program_accounts_with_config(&restaking_program_id, config)
                .await?;

            let mut ixs = Vec::with_capacity(ncn_vault_tickets.len());
            for (_ncn_vault_ticket_addr, ncn_vault_ticket_acc) in ncn_vault_tickets {
                let ncn_vault_ticket =
                    NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_acc.data)?;

                let operator_vault_ticket_addr = OperatorVaultTicket::find_program_address(
                    &restaking_program_id,
                    &Pubkey::from_str(operator_address.as_str()).unwrap(),
                    &ncn_vault_ticket.vault,
                )
                .0;

                if let Err(_e) = rpc_client.get_account(&operator_vault_ticket_addr).await {
                    let mut ix_builder = InitializeOperatorVaultTicketBuilder::new();
                    ix_builder
                        .config(restaking_config_addr)
                        .operator(Pubkey::from_str(operator_address.as_str()).unwrap())
                        .vault(ncn_vault_ticket.vault)
                        .admin(keypair.pubkey())
                        .operator_vault_ticket(operator_vault_ticket_addr)
                        .payer(keypair.pubkey());
                    let mut ix = ix_builder.instruction();
                    ix.program_id = restaking_program_id;

                    ixs.push(ix);
                }
            }

            let blockhash = rpc_client.get_latest_blockhash().await?;
            let tx = Transaction::new_signed_with_payer(
                &ixs,
                Some(&keypair.pubkey()),
                &[keypair.clone()],
                blockhash,
            );
            let result = rpc_client.send_and_confirm_transaction(&tx).await?;

            info!("Transaction confirmed: {:?}", result);

            if !backup_snapshots_dir.exists() {
                info!(
                    "Creating backup snapshots directory at {}",
                    backup_snapshots_dir.display()
                );
                std::fs::create_dir_all(&backup_snapshots_dir)?;
            }

            let try_catchup = tip_router_operator_cli::solana_cli::catchup(
                cli.rpc_url.to_owned(),
                cli.localhost_port,
            );
            if let Err(ref e) = &try_catchup {
                datapoint_error!(
                    "tip_router_cli.main",
                    ("operator_address", cli.operator_address, String),
                    ("status", "error", String),
                    ("error", e.to_string(), String),
                    ("state", "bootstrap", String),
                    "cluster" => &cli.cluster,
                );
                error!("Failed to catch up: {}", e);
            }

            if let Ok(command_output) = &try_catchup {
                info!("{}", command_output);
            }

            tokio::spawn(async move {
                loop {
                    datapoint_info!(
                        "tip_router_cli.heartbeat",
                        ("operator_address", operator_address, String),
                        "cluster" => cluster,
                    );
                    sleep(Duration::from_secs(cli.heartbeat_interval_seconds)).await;
                }
            });

            // Check for new meta merkle trees and submit to NCN periodically
            tokio::spawn(async move {
                let keypair_arc = keypair.clone();
                loop {
                    if let Err(e) = submit_recent_epochs_to_ncn(
                        &rpc_client_clone,
                        &keypair_arc,
                        &ncn_address,
                        &tip_router_program_id,
                        &tip_distribution_program_id,
                        &priority_fee_distribution_program_id,
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

            // Claim tips and emit metrics
            let file_mutex = Arc::new(Mutex::new(()));

            // Run claims if enabled
            if claim_tips_metrics {
                let cli_clone = cli.clone();
                let rpc_client_clone = rpc_client.clone();
                let file_path_ref = claim_tips_epoch_filepath.clone();
                let file_mutex_ref = file_mutex.clone();

                tokio::spawn(async move {
                    loop {
                        // Get current epoch
                        let current_epoch = match rpc_client_clone.get_epoch_info().await {
                            Ok(epoch_info) => epoch_info.epoch,
                            Err(_) => {
                                // If we can't get the epoch, wait and retry
                                sleep(Duration::from_secs(60)).await;
                                continue;
                            }
                        };
                        for epoch_offset in 0..claim_tips_epoch_lookback {
                            let epoch_to_emit = current_epoch
                                .checked_sub(epoch_offset)
                                .expect("Epoch underflow")
                                .checked_sub(1)
                                .expect("Epoch overflow");

                            info!("Emitting Claim Metrics for epoch {}", epoch_to_emit);
                            let cli_ref = cli_clone.clone();
                            if epoch_to_emit >= legacy_tip_router_operator_cli::PRIORITY_FEE_MERKLE_TREE_START_EPOCH {
                            match emit_claim_mev_tips_metrics(
                                &cli_ref,
                                epoch_to_emit,
                                tip_distribution_program_id,
                                priority_fee_distribution_program_id,
                                tip_router_program_id,
                                ncn_address,
                                &file_path_ref,
                                &file_mutex_ref,
                            )
                            .await
                            {
                                Ok(_) => {
                                    info!(
                                        "Successfully emitted claim metrics for epoch {}",
                                        epoch_to_emit
                                    );
                                }
                                Err(e) => {
                                    error!(
                                        "Error emitting claim metrics for epoch {}: {}",
                                        epoch_to_emit, e
                                    );
                                }
                            }
                            } else {
                                match legacy_tip_router_operator_cli::claim::emit_claim_mev_tips_metrics(
                                    &cli_ref.as_legacy(),
                                    epoch_to_emit,
                                    tip_distribution_program_id,
                                    tip_router_program_id,
                                    ncn_address,
                                    &file_path_ref,
                                    &file_mutex_ref,
                                ).await
                                {
                                    Ok(_) => {
                                        info!(
                                            "Successfully emitted claim metrics for epoch {}",
                                            epoch_to_emit
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            "Error emitting claim metrics for epoch {}: {}",
                                            epoch_to_emit, e
                                        );
                                    }
                                }
                            }
                        }

                        info!("Sleeping for 30 minutes before next emit claim cycle");
                        sleep(Duration::from_secs(1800)).await;
                    }
                });
            }

            if claim_tips {
                let cli_clone = cli.clone();
                let rpc_client_clone = rpc_client.clone();

                tokio::spawn(async move {
                    loop {
                        // Get current epoch
                        let current_epoch = match rpc_client_clone.get_epoch_info().await {
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
                        for epoch_offset in 0..claim_tips_epoch_lookback {
                            let epoch_to_process = current_epoch
                                .checked_sub(epoch_offset)
                                .expect("Epoch underflow")
                                .checked_sub(1)
                                .expect("Epoch overflow");
                            let cli_ref = cli_clone.clone();
                            let file_path_ref = claim_tips_epoch_filepath.clone();
                            let file_mutex_ref = file_mutex.clone();

                            // Create a task for each epoch and add its handle to our vector
                            let handle = tokio::spawn(async move {
                                info!("Processing claims for epoch {}", epoch_to_process);
                                let result = claim_mev_tips_with_emit(
                                    &cli_ref,
                                    epoch_to_process,
                                    tip_distribution_program_id,
                                    priority_fee_distribution_program_id,
                                    tip_router_program_id,
                                    ncn_address,
                                    Duration::from_secs(3600),
                                    &file_path_ref,
                                    &file_mutex_ref,
                                )
                                .await;

                                match result {
                                    Err(e) => {
                                        error!(
                                            "Error claiming tips for epoch {}: {}",
                                            epoch_to_process, e
                                        );
                                    }
                                    Ok(_) => {
                                        info!(
                                            "Successfully processed claims for epoch {}",
                                            epoch_to_process
                                        );
                                    }
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
                &priority_fee_distribution_program_id,
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
            priority_fee_distribution_program_id,
            tip_router_program_id,
            epoch,
            set_merkle_roots,
        } => {
            let meta_merkle_tree_path = meta_merkle_tree_path(epoch, &cli.get_save_path());

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
                &priority_fee_distribution_program_id,
                cli.submit_as_memo,
                set_merkle_roots,
                cli.vote_microlamports,
                &cli.cluster,
            )
            .await?;
        }
        Commands::ClaimTips {
            tip_router_program_id,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
            ncn_address,
            epoch,
        } => {
            info!("Claiming tips...");
            let claim_tips_epoch_filepath = cli.claim_tips_epoch_filepath.clone();
            let file_mutex = Arc::new(Mutex::new(()));
            claim_mev_tips_with_emit(
                &cli,
                epoch,
                tip_distribution_program_id,
                priority_fee_distribution_program_id,
                tip_router_program_id,
                ncn_address,
                Duration::from_secs(3600),
                &claim_tips_epoch_filepath,
                &file_mutex,
            )
            .await?;
        }
        Commands::CreateStakeMeta {
            epoch,
            slot,
            tip_distribution_program_id,
            priority_fee_distribution_program_id,
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
                &priority_fee_distribution_program_id,
                &tip_payment_program_id,
                &save_path,
                save,
                &cli.cluster,
            );
        }
        Commands::CreateMerkleTreeCollection {
            tip_router_program_id,
            ncn_address,
            epoch,
            save,
        } => {
            // Load the stake_meta_collection from disk
            let stake_meta_collection = read_stake_meta_collection(
                epoch,
                &cli.get_save_path().join(stake_meta_file_name(epoch)),
            );
            let config = get_ncn_config(&rpc_client, &tip_router_program_id, &ncn_address).await?;
            // Tip Router looks backwards in time (typically current_epoch - 1) to calculated
            //  distributions. Meanwhile the NCN's Ballot is for the current_epoch. So we
            //  use epoch + 1 here
            let ballot_epoch = epoch.checked_add(1).unwrap();
            let fees = config.fee_config.current_fees(ballot_epoch);
            let protocol_fee_bps = config.fee_config.adjusted_total_fees_bps(ballot_epoch)?;

            // Generate the merkle tree collection
            create_merkle_tree_collection(
                cli.operator_address,
                &tip_router_program_id,
                stake_meta_collection,
                epoch,
                &ncn_address,
                protocol_fee_bps,
                fees.priority_fee_distribution_fee_bps(),
                &save_path,
                save,
                &cli.cluster,
            );
        }
        Commands::CreateMetaMerkleTree { epoch, save } => {
            // Load the stake_meta_collection from disk
            let merkle_tree_collection = read_merkle_tree_collection(
                epoch,
                &cli.get_save_path()
                    .join(merkle_tree_collection_file_name(epoch)),
            );

            create_meta_merkle_tree(
                cli.operator_address,
                merkle_tree_collection,
                epoch,
                &save_path,
                save,
                &cli.cluster,
            );
        }
    }
    Ok(())
}

#![allow(clippy::integer_division)]
use ::{
    anyhow::Result,
    clap::Parser,
    log::{error, info},
    solana_metrics::{datapoint_error, datapoint_info, set_host_id},
    solana_rpc_client::nonblocking::rpc_client::RpcClient,
    solana_sdk::{pubkey::Pubkey, signer::keypair::read_keypair_file},
    std::process::Command,
    std::{str::FromStr, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        account_analyzer,
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

            if !backup_snapshots_dir.exists() {
                info!(
                    "Creating backup snapshots directory at {}",
                    backup_snapshots_dir.display()
                );
                std::fs::create_dir_all(&backup_snapshots_dir)?;
            }

            let operator_address = cli.operator_address.clone();
            let cluster = cli.cluster.clone();

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
                let keypair_arc = Arc::new(keypair);
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
        Commands::AnalyzeAccounts {
            tip_distribution_program_id,
            tip_router_program_id,
            ncn_address,
            epoch,
            json_data,
        } => {
            info!("Analyzing accounts...");
            let json_808_data = r#"
[
  {
    "vote_account": "KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag",
    "mev_revenue": "128250941158",
    "claim_status_account": "8YBTPmJ7c12od7KFJzhGuhktVxVkCJMcs99bZhRwM37i"
  },
  {
    "vote_account": "QhyTEHb5JkMBki8Lq1npsaixefUyMXWJtbxK6jNjxnn",
    "mev_revenue": "8330078573",
    "claim_status_account": "5uM33U31VsANx5CD15Dh1SVZrkctsApQrSZTd2QRzrV4"
  },
  {
    "vote_account": "StepeLdhJ2znRjHcZdjwMWsC4nTRURNKQY8Nca82LJp",
    "mev_revenue": "10468294325",
    "claim_status_account": "5dw88wDEe3cfUEUq1aexXdy7eeP5j6pJ6SU2i8tcSRfZ"
  },
  {
    "vote_account": "VoTEJDVw84uZvDWcMXYgfNAVcLETHsPyPEc2nTpwZPa",
    "mev_revenue": "587857644",
    "claim_status_account": "H4Eq4jgELj8Ys6Tk9QMarXXwcQc9VBt76sazZUwhY6kn"
  },
  {
    "vote_account": "VoteMYitKq7mruk9QPJRUgryYbSkyZKBuvnL1VTgoMq",
    "mev_revenue": "116348706",
    "claim_status_account": "7Pe4Kk9YsBLzkna7Lmdinx1gnPcR8cyHkJfqsAocp94S"
  },
  {
    "vote_account": "iZADA4YKVRJZJaDUV3j79DzyK4VJkK3DGTfvvqvbC1K",
    "mev_revenue": "121746521543",
    "claim_status_account": "FkvXJn2cxh6XP9rDPWPtHtaFQm6q3u4JF62qSQXEQVXZ"
  },
  {
    "vote_account": "irKsY8c3sQur1XaYuQ811hzsEQJ5Hq3Yu3AAoXYnp8W",
    "mev_revenue": "2170126571",
    "claim_status_account": "E6qhr2yD25TJCpCbE62HbnLoRAncc1qXae6BbiHfoK44"
  },
  {
    "vote_account": "oDDiLXv87uRfbAB8PZthCtQyqof2Jomv7fpTeoBp6AY",
    "mev_revenue": "1570501897",
    "claim_status_account": "5AiPM5ZK1VHbiCnUdacLdFURumw8CJs1qH15KoqN4gL3"
  },
  {
    "vote_account": "oixpqSNX7CKWHw93ViA8u1CcLzZXDmacKJjV4AvxMZE",
    "mev_revenue": "92021779981",
    "claim_status_account": "FHr4fvniww7kVFfF4ksgBFi9cKo34K3K1AgNfHvBFkSV"
  },
  {
    "vote_account": "privSGy4XbFCjzEkdLXssV8xMWRWWiWbJeDzh1emUyL",
    "mev_revenue": "44317114",
    "claim_status_account": "EGWHMzp4K8CnquN6RG3g9BS1efoVT7pvsEMG12Se81Aj"
  },
  {
    "vote_account": "reyYoUdFgtDLxAWW1hyn5xk2PHstA3j8zUrerQi9Ayq",
    "mev_revenue": "1443007378",
    "claim_status_account": "2LoqGbiowuy8K4GAN2AMxVkbfPHbyubGGuyofQJ8eQz1"
  },
  {
    "vote_account": "2PEyBgsPYBQ8pMdXQtEaPGNqWQHE9GCnmV2tTVN4GMru",
    "mev_revenue": "2741741537",
    "claim_status_account": "8VvwTVYPEtWY5YSHr3LYphxP78NMSkYCmqkBvDT68cap"
  },
  {
    "vote_account": "2qD6yvLwy3ckWxsS1iQwrkCjLgcvH1u9PLu5m9KRRn5x",
    "mev_revenue": "4917131428",
    "claim_status_account": "rnX5QKchUgDEnbAiHVDGJZbYGGyYbbvardDG6LDf8PA"
  },
  {
    "vote_account": "2uXzxR2EZVaHE3CuaDaUJ8C9Qr54LMfwkY5BtRPAzbPE",
    "mev_revenue": "109784027438",
    "claim_status_account": "BdzWD35CWYeaYYAfMtAmru6HBfgFCgnwdfKESJ3hPWqL"
  },
  {
    "vote_account": "33hurzEz6aEnzfESL6pnNyR6DCgcKzssT1pwSzDCBTRQ",
    "mev_revenue": "85770932452",
    "claim_status_account": "5h9eweDkbLn9f21oxH7GAmszqCQTQzr1T1sWCiJTx3sL"
  },
  {
    "vote_account": "37BPVW1Ne1XHrzK15xguAS2BTdobVfThDzTE2mv8SsnJ",
    "mev_revenue": "26014913",
    "claim_status_account": "9pdL9wRA3WW5tdJjZDeR88C8CdQ7yS8yYvfTL36hiknp"
  },
  {
    "vote_account": "3ZYJxzCeweSoh2Jj7oCgencFs9y27iKmXJeqYapje1cj",
    "mev_revenue": "248004537893",
    "claim_status_account": "3Yuwm9vEZCMcXjzd5NXcRJ9TTBnsuHDirwCbqCtPaWp8"
  },
  {
    "vote_account": "3jkJVgfz1zrHSy6YLK6g96eTj49kCnDj2i8AbbKLZhkk",
    "mev_revenue": "632365469",
    "claim_status_account": "6pTHoHApa8aP7Kv8iCNHpsmmXNs47CDrmJmdGoP99p86"
  },
  {
    "vote_account": "4T799AaK9YT7zBtVqYZEnCY5ihUF5XaEYemwr5EnozoQ",
    "mev_revenue": "535590841",
    "claim_status_account": "CoJigm6TWrsDyUCn5AW8wcmFCsevKFRgTjhzg4yLLGad"
  },
  {
    "vote_account": "4jx1b7HCN9nCxygP3hruC85BxcYndhxby4hkNexuHvxT",
    "mev_revenue": "5585858743",
    "claim_status_account": "6ZNw8U8mPRmZPm9QQAG9SjvgJV92vKYbM3rdcsFy3zm7"
  },
  {
    "vote_account": "5wYHvcKbCHPsT9dhEZaVZzoVY1qA7bosKRzKczpcaXpq",
    "mev_revenue": "1183846504",
    "claim_status_account": "EBkXrLZrA7f6PV2ALT6oAhGBiZtiZgP6XzLGpZMHuvKh"
  },
  {
    "vote_account": "6F5xdRXh2W3B2vhte12VG79JVUkUSLYrHydGX1SAadfZ",
    "mev_revenue": "41880406303",
    "claim_status_account": "CympfNrGg2RLPgDZezCs9FXntneJpxbNsn4kt4UFPoYZ"
  },
  {
    "vote_account": "6hTLQ5HSdWcpZkbXmZxXaGjCgTh7zh8UeWKWKgGE1BPp",
    "mev_revenue": "2634833850",
    "claim_status_account": "CotzKQegfXEu1Fov48YDodYoycithfReae9duy3XRMr4"
  },
  {
    "vote_account": "6jzDwKeR21EFHwaRgZMefMxJ9D2vnQRqfYxkpUuJppPh",
    "mev_revenue": "122694556881",
    "claim_status_account": "6kKiJVfZ6qWtBzazzoQ8MmDcjvRNUygvmk2AA3fx6Li5"
  },
  {
    "vote_account": "6tgtejPHUHR1pECzXqQT8EHZqnKCWZFSqdZXDyBaKe3b",
    "mev_revenue": "12179208836",
    "claim_status_account": "B7WCRb6vw1b4cK3MmsvqvrQBH9KsDx8Wu9ANfwaRrDkL"
  },
  {
    "vote_account": "72LbWsZFEyB7xrB9ggeoPUrSw2vzPEnsPJJHZo1svkM7",
    "mev_revenue": "110662983952",
    "claim_status_account": "5mvaecURvCqMCrNmHwbhEP9aGv4PoPXVCEBPM9ff9iPZ"
  },
  {
    "vote_account": "74pfDmYto6aAqCzFH1mNJ8NxF7A4LQ4cXkipGwgjY39u",
    "mev_revenue": "56477055761",
    "claim_status_account": "4dp4a5cVuTuRawYhmScWfQVmgipQrVXkVJLtTxqwPkBL"
  },
  {
    "vote_account": "7jPqpHuN5v59dtBom2tjmYEfi6WaM4sFtJeTD6fzhcdS",
    "mev_revenue": "45833923433",
    "claim_status_account": "BLoCmgwfibM2niH6LFLJfuupSCPDetVU4qbgrwn7UneL"
  },
  {
    "vote_account": "7opSZGmevWhRDyLt5Wu38FZFjUyredGmMki4DNmxDnjd",
    "mev_revenue": "33435980947",
    "claim_status_account": "rZzzuDhFqiWfUsHQrq6PyLviNG9zXk5v8gvdgpRn5rd"
  },
  {
    "vote_account": "7xENfwKCajMB5aVTgmTB6h7d7Su91wTcnfMjoAQCMvKq",
    "mev_revenue": "123415631920",
    "claim_status_account": "5WZBc63zbYtfADauCjCuPLqH3WKBtuSnzjX8CAx4ys4m"
  },
  {
    "vote_account": "8FPz3JG4E3HVXxGbPZVibarva4AGXSZWx3qKLUS5uFtN",
    "mev_revenue": "257295004",
    "claim_status_account": "BX64dYh8fBmNDV5tRsNA9ja1XELJszwHTavX3pw6ckXV"
  },
  {
    "vote_account": "8Pep3GmYiijRALqrMKpez92cxvF4YPTzoZg83uXh14pW",
    "mev_revenue": "159206649350",
    "claim_status_account": "zUeTw5JtePJS7Yh1FndP5WwyTFsbNuBUjayFtKnyUR3"
  },
  {
    "vote_account": "8mHUDJjzPo2AwJp8SHKmG9rk9ftWTp7UysqYz36cMpJe",
    "mev_revenue": "105616897844",
    "claim_status_account": "HTLbutPUWWSkigWREYos8weW1YuVX3uuRh2Y8h8JMyu8"
  },
  {
    "vote_account": "8r4Fu6M8brgnL456RJfwxk8kN4iw1LgczfuXeuG1g4px",
    "mev_revenue": "11013603288",
    "claim_status_account": "2bgxJcS9FkZ81eX14E1o2wYkcKQKJ4phoyBUug6eh2PL"
  },
  {
    "vote_account": "8wTSPukwTAzNzEYyUdc8UiKkTg1hNtZ1xLum7o1Ne6wr",
    "mev_revenue": "84672358707",
    "claim_status_account": "DUfR488SSwx7BYTjFXJVduJFMxt4F5ABkMXJEUacsq6n"
  },
  {
    "vote_account": "8zHJtME22tiY3UsSHtDJXo2J8hUfwikBxXNbVqQzA92r",
    "mev_revenue": "31332710023",
    "claim_status_account": "FRMJTioAcFgrK1sMJTwWLqQziyfUJ8HZWE1agpntHQ9V"
  },
  {
    "vote_account": "9Diao4uo6NpeMud7t5wvGnJ3WxDM7iaYxkGtJM36T4dy",
    "mev_revenue": "83286020979",
    "claim_status_account": "AdjZHsCUYVETXVbkUsSrPGY2a19v1Zc3BoNM572WoK3y"
  },
  {
    "vote_account": "9QU2QSxhb24FUX3Tu2FpczXjpK3VYrvRudywSZaM29mF",
    "mev_revenue": "267702558658",
    "claim_status_account": "Aue2bC9JYDPuCqyUJVvQ1fc8bSQ2SkfoZj37i9QYJSpb"
  },
  {
    "vote_account": "AZoCYB4VgoM9DR9f1ZFcBn8xPSbtbqoxZnKJR7tkvEoX",
    "mev_revenue": "249387620402",
    "claim_status_account": "FPjEXY1fZJ9LyCLrArfxsoamSAc7DpWqsPyKufaWSXcx"
  },
  {
    "vote_account": "At2rZHk554qWrjcmdNkCQGp8i4hdKLf52EXMrDmng5ab",
    "mev_revenue": "112907620818",
    "claim_status_account": "EWN16dERwYXxp1ShfPD4YipSPWK2oSDjYutj2oB1VHsj"
  },
  {
    "vote_account": "B38JgkTi7Fu2Uxk8JzNw4M7aMhVxzGu2fsRqHNScPkCQ",
    "mev_revenue": "141009621",
    "claim_status_account": "FRVdDkYYxf37eGsbr5MFny6YjgP5iUJvsHCmioDXn3ej"
  },
  {
    "vote_account": "BLADE1qNA1uNjRgER6DtUFf7FU3c1TWLLdpPeEcKatZ2",
    "mev_revenue": "99054608953",
    "claim_status_account": "9WZs7UJuMe8S6vwWZKvPaaUbDxCtZaZnQ2Q6yvw4EBGP"
  },
  {
    "vote_account": "BU3ZgGBXFJwNTrN6VUJ88k9SJ71SyWfBJTabYqRErm4F",
    "mev_revenue": "85235047372",
    "claim_status_account": "7dDD7tZr1voEQjg4jDjD3LKnWtFLVjPJd7DcJymrHENZ"
  },
  {
    "vote_account": "BbM5kJgrwEj3tYFfBPnjcARB54wDUHkXmLUTkazUmt2x",
    "mev_revenue": "6747153848",
    "claim_status_account": "DKFSSTTKYHeEV2fRN5PM9JWnTerFcg7yzw7uMcMjV3Ff"
  },
  {
    "vote_account": "BhREyEsP3YAtQbTCrKcXgTNTeaq9gdjWji3Nz4d8Q1P2",
    "mev_revenue": "125100235025",
    "claim_status_account": "8xDgabi7PQaDbyk3DmtQYyjsZVSC6h7JFhHX7KsbhpnH"
  },
  {
    "vote_account": "BkSS8kGUNcQkTgEKMmBhHxGVLdzw43EAzDYpZqyyxFrT",
    "mev_revenue": "58472427881",
    "claim_status_account": "CUvQMfjFXawYMqHwkTWFV5B2HhMe553jPxyvtP387rZS"
  },
  {
    "vote_account": "Bkskrv38Kn7zJR5mvmbCDGn2M4Jyhzt2ZqwQXV6rYnXa",
    "mev_revenue": "35460223302",
    "claim_status_account": "9Li19aK6ZrmRTeBkaQimCwNkjRZVSeN2FYRqqbGq4zQJ"
  },
  {
    "vote_account": "Bwkz1ddKoGE8hgiSV6HZLXi9RBLqfBi3HZb2QujzVGgz",
    "mev_revenue": "16807856393",
    "claim_status_account": "4mFZ7Aanh1j4FfNtY5ZFEYN7np4NHWhpfeQk1mv6Bayu"
  },
  {
    "vote_account": "CtiiCQbRh13cqorWaEimroRznTL2qTytNhzYz53BCnbq",
    "mev_revenue": "2849968227",
    "claim_status_account": "RVtuwrEA95htnn2VVsPZD7AtfnNbvvEjuGm8Qg5nHnH"
  },
  {
    "vote_account": "CxFH1pqJnEmyaE4wEwqdqMKpQMpkmdaMxhS7SzpHokA8",
    "mev_revenue": "52882665021",
    "claim_status_account": "CuirPq6icm4PA5FWhDZLXSiympHu62MEAp25ypZiZZ3X"
  },
  {
    "vote_account": "DLKjd8DJc9NajCaHPeQL6BnhPi3a4BZm7zCdVF3MzDRZ",
    "mev_revenue": "16988097625",
    "claim_status_account": "3bBx19EkM1aUtwCzHT19Yow9fVpeAb7PqGoqpUGKLywc"
  },
  {
    "vote_account": "DQ7D6ZRtKbBSxCcAunEkoTzQhCBKLPdzTjPRRnM6wo1f",
    "mev_revenue": "50405943955",
    "claim_status_account": "Gdkm845gU1J6WLKCeYYVixV6HSWKrgTxUuYxJ28oZBL7"
  },
  {
    "vote_account": "DdCNGDpP7qMgoAy6paFzhhak2EeyCZcgjH7ak5u5v28m",
    "mev_revenue": "503192040697",
    "claim_status_account": "GhRQYCdrJnKSqcsem7hQCq4hrA6NU6kAft2eQNTEVWe2"
  },
  {
    "vote_account": "E9W5kU2fnha9yp4RmFZgNNsRUvy6oKnB9ZyR9LC81WaE",
    "mev_revenue": "112355752842",
    "claim_status_account": "FkR1zMt45vsXpej1chHpQPYXoorE7mktWp23qDifXfGG"
  },
  {
    "vote_account": "EXhYxF25PJEHb3v5G1HY8Jn8Jm7bRjJtaxEghGrUuhQw",
    "mev_revenue": "68576440887",
    "claim_status_account": "sgz4D5wx4s6BeDbtXB5GFxLsPM78yyzJkEKNWFEyMaZ"
  },
  {
    "vote_account": "Eajfs6oXGGkvjYsxkQZZJcDCLLkUajaHizfgg2xTsqyd",
    "mev_revenue": "9588471814",
    "claim_status_account": "9Wciw6XUcUtPaoJfesSBrTR3Hgg1ZD1RbmgTXRcEHL85"
  },
  {
    "vote_account": "EcEowA4GKDsdVBF9PNAZa6c9M4WgYG8y4GnpZSUaqioS",
    "mev_revenue": "74048125",
    "claim_status_account": "5H7Tse5m2FLcPNNYUqB8kCFwbjaKQVp5DRgE2zhJd1jJ"
  },
  {
    "vote_account": "EcLPNfLFgCkbcTuvdeQ85pnQMgAfBDqi2dkoNVPrSyr5",
    "mev_revenue": "39715646052",
    "claim_status_account": "AejZVseyBzceUScpKyhCrB3sBoz3ma6Upo2JccjrUjbB"
  },
  {
    "vote_account": "EpRvips2doUUdxvs3Qhf4MCLqVeJEPu47Aci5QbBXASV",
    "mev_revenue": "4566315126",
    "claim_status_account": "C6JrbD2XVdYtyzziKva8xYZem9u631LjMR6PeaeDqwqg"
  },
  {
    "vote_account": "FjkSLYmi6BJAJQn1iSLUGrPrBQjMaD4y1DVdnv3yaTsX",
    "mev_revenue": "190390430585",
    "claim_status_account": "A9L9NCtMtQkw27GBEL43HGh3oCny5RRqaxV95qPHvNqU"
  },
  {
    "vote_account": "G9x1mqewTeVnXLmv3FamYD5tq1AdS395RHH3MLQPj6TY",
    "mev_revenue": "222655300743",
    "claim_status_account": "EQFZ2HyUm4raJPhTRBVMUn7q4iKMhQHk6PZB32SRnnp5"
  },
  {
    "vote_account": "GhBWWed6j9tXLEnKiw9CVDHyQCYunAVGnssrbYxbBmFm",
    "mev_revenue": "8449433671",
    "claim_status_account": "3NQZQSxcGhXojHtxYdW9BPJK65SzcT9MQetsSABY2Asu"
  },
  {
    "vote_account": "GioetmC79nLRnN7VDfHaq8coWAEFPJKu9py59uUqdV5U",
    "mev_revenue": "2591009461",
    "claim_status_account": "5msh2jhinq8DxVN4KXvaJoQbYGcepbsS5ZWzFs2UHZSD"
  },
  {
    "vote_account": "GvZEwtCHZ7YtCkQCaLRVEXsyVvQkRDhJhQgB6akPme1e",
    "mev_revenue": "92292661329",
    "claim_status_account": "2hfWL4sWB8P4sqjUxQNvDvLFYBF6Sw49VgvkmQQnBAJ6"
  },
  {
    "vote_account": "H43AYFsvhNuALQieHpLXefp1ECgEBT6oVnS4EcTsC25C",
    "mev_revenue": "27844669418",
    "claim_status_account": "8jHHwhRmL6DrWwC6f3hUrmKxYbzPNmpQqQP6xAqXUdBL"
  },
  {
    "vote_account": "H74qox1GASBWd94FWMyy6GVAbRVLf9SAMgbJ1tzSUAst",
    "mev_revenue": "231027486819",
    "claim_status_account": "7q8T1V9zsJ6zqf4NSuqUBj5T6zEmwa6diB6MtummwYDo"
  },
  {
    "vote_account": "HDc84gs3CtqhebHycmoDpc5n2y3CFfd5GqYZkr2XiBMR",
    "mev_revenue": "114388759230",
    "claim_status_account": "8DnE5YxdKcrGv46D8P3HMEFr8KJJPpvCFMGm8LMc7WxR"
  },
  {
    "vote_account": "HZKopZYvv8v6un2H6KUNVQCnK5zM9emKKezvqhTBSpEc",
    "mev_revenue": "209558075823",
    "claim_status_account": "F9JyM5GB3K933r9KSBbKHuG9hCQh7opyzW5s7XEebFdG"
  },
  {
    "vote_account": "HhYEE3dAShc3772wEiy73XDYnLjVxyBL8eAWKyRcF14y",
    "mev_revenue": "69801367495",
    "claim_status_account": "7AQWats4kdCHQWWf8qKCZRQNKPtFY1BfHHN2NpCAtmXe"
  },
  {
    "vote_account": "HxYHGzR58gyf6c4JAX85eK8GVuaZU2zne4be82Lq9SBQ",
    "mev_revenue": "94921236362",
    "claim_status_account": "6jZWxRgPZb6V5HcbhA7Q6K598cEi2NuqQDtn7wLLDsu1"
  },
  {
    "vote_account": "JDMq8hxZnad2smKLGkFbfg8zVMZHKQcMugD4tMR9u2da",
    "mev_revenue": "79334182045",
    "claim_status_account": "8DajTPFaoT3SKdQyBV5ciFXRve1TbN6SyDbqPd2n8VjL"
  }
] 
"#;

            let json_809_data = r#"
[
  {
    "vote_account": "HDc84gs3CtqhebHycmoDpc5n2y3CFfd5GqYZkr2XiBMR",
    "mev_revenue": "88628734177",
    "claim_status_account": "6shBuKQaqJJjXfiAX1LUcFsDFnXSTcJ8n4Mnd1QSJEaC"
  },
  {
    "vote_account": "3jkJVgfz1zrHSy6YLK6g96eTj49kCnDj2i8AbbKLZhkk",
    "mev_revenue": "713326517",
    "claim_status_account": "Cqf7AyN3htDmEovnqq8LNewjiGe86Pf9nC4sqH5pBofH"
  },
  {
    "vote_account": "3ZYJxzCeweSoh2Jj7oCgencFs9y27iKmXJeqYapje1cj",
    "mev_revenue": "210651667338",
    "claim_status_account": "6usrQSirxw4Wn8kwk35XrWyprovoBjhnBPqssVHCY4a1"
  },
  {
    "vote_account": "GAoCBDE9ABRYmhWUREDp4po4L8JgCPeQsPS77p1mx3WR",
    "mev_revenue": "5217556406",
    "claim_status_account": "5MgifEnrd8SCvZAcYYEo9NfU4WNoKCa6yDujKstD3UWF"
  },
  {
    "vote_account": "reyYoUdFgtDLxAWW1hyn5xk2PHstA3j8zUrerQi9Ayq",
    "mev_revenue": "908848820",
    "claim_status_account": "DWjaUYSQDiFPwo7VGC1kXNjNePKUAs7bNMHnDL4rsn45"
  },
  {
    "vote_account": "BbM5kJgrwEj3tYFfBPnjcARB54wDUHkXmLUTkazUmt2x",
    "mev_revenue": "8484030175",
    "claim_status_account": "CJgvti4iVEoZVZYomNg7UpWfAR5kfiha2kdzwc2m14is"
  },
  {
    "vote_account": "CxFH1pqJnEmyaE4wEwqdqMKpQMpkmdaMxhS7SzpHokA8",
    "mev_revenue": "37808301347",
    "claim_status_account": "48MgrvzsbsKeG7pV8JfBjmt1n4Bdi8oyUx7ENLVxmJ1F"
  },
  {
    "vote_account": "DdCNGDpP7qMgoAy6paFzhhak2EeyCZcgjH7ak5u5v28m",
    "mev_revenue": "389024207571",
    "claim_status_account": "7fNCKNGFZ1e8vLwevj5RzxUEbieVAipDgsauqmvFCUnA"
  },
  {
    "vote_account": "oDDiLXv87uRfbAB8PZthCtQyqof2Jomv7fpTeoBp6AY",
    "mev_revenue": "2046928004",
    "claim_status_account": "FpqBNLzhBWyZz4ra7Fh6v7u58gD6dQvpSDne8mxSwrZZ"
  },
  {
    "vote_account": "BhREyEsP3YAtQbTCrKcXgTNTeaq9gdjWji3Nz4d8Q1P2",
    "mev_revenue": "92032732252",
    "claim_status_account": "S1r5tvh14aHw2PHLtoabW9azZC6eL6jvMJ3dLNfH6bs"
  },
  {
    "vote_account": "oixpqSNX7CKWHw93ViA8u1CcLzZXDmacKJjV4AvxMZE",
    "mev_revenue": "71072928572",
    "claim_status_account": "5VCup7jjYjUz1pEznetaLLvcf6kw15F7sfY1B3XKzs6"
  },
  {
    "vote_account": "H74qox1GASBWd94FWMyy6GVAbRVLf9SAMgbJ1tzSUAst",
    "mev_revenue": "207616917281",
    "claim_status_account": "FJ6Vp2BDRpSkLMhYoGxfuCaVUX92hrTJstt2feTd78w"
  },
  {
    "vote_account": "8zHJtME22tiY3UsSHtDJXo2J8hUfwikBxXNbVqQzA92r",
    "mev_revenue": "29459103241",
    "claim_status_account": "F5MiSMj7uqwS58SoWG4P7Xc5hK71Ak8mSzftXxoxCdra"
  },
  {
    "vote_account": "Dh4K8fNV6pRFZtbzQnP5a5HmyBPb2kmxvWiYmc5fJMvj",
    "mev_revenue": "1171196320",
    "claim_status_account": "32U3PwqthEmtD6RmYXUiQgXPZ9p7Ustcw5z15NWc2zE2"
  },
  {
    "vote_account": "AZoCYB4VgoM9DR9f1ZFcBn8xPSbtbqoxZnKJR7tkvEoX",
    "mev_revenue": "213127666642",
    "claim_status_account": "3j37bai5n1x56rmxV73epJmcTCLL1LYPSpV8cU3uNBaS"
  },
  {
    "vote_account": "7xENfwKCajMB5aVTgmTB6h7d7Su91wTcnfMjoAQCMvKq",
    "mev_revenue": "114989457459",
    "claim_status_account": "GdyQEMF4CfyK27c7fsS4hq6GgL7MefVjDKSE7tqpRSo9"
  },
  {
    "vote_account": "9Diao4uo6NpeMud7t5wvGnJ3WxDM7iaYxkGtJM36T4dy",
    "mev_revenue": "77802060063",
    "claim_status_account": "HmpM5QjePcFUybujsgbysA1uEcbc5twR2Lf6qb8CA1iG"
  },
  {
    "vote_account": "6F5xdRXh2W3B2vhte12VG79JVUkUSLYrHydGX1SAadfZ",
    "mev_revenue": "41108239200",
    "claim_status_account": "8os2nBzBkSBA55BUCc6YecppTy6eUzrRHiNuBiMVDWvD"
  },
  {
    "vote_account": "72LbWsZFEyB7xrB9ggeoPUrSw2vzPEnsPJJHZo1svkM7",
    "mev_revenue": "87242400468",
    "claim_status_account": "CkWY2FVvAspvjdsmd1fxuNAPAwEXcnwjrQbRppN6tgPj"
  },
  {
    "vote_account": "StepeLdhJ2znRjHcZdjwMWsC4nTRURNKQY8Nca82LJp",
    "mev_revenue": "8296508377",
    "claim_status_account": "848HYQCKANPARXwoDFAeWyncTH5iFTDp4AtGCkyNKsVf"
  },
  {
    "vote_account": "74pfDmYto6aAqCzFH1mNJ8NxF7A4LQ4cXkipGwgjY39u",
    "mev_revenue": "49343888257",
    "claim_status_account": "47sCYzbVktSQvyrzCLkoq6Zwg5FYMYcD72mMmLb1yL5B"
  },
  {
    "vote_account": "ApcUbFDskBMrYJqu9orEnPSq5uMAH1YjhwR4DP2JwKT8",
    "mev_revenue": "2011030152",
    "claim_status_account": "4KjzxvM5M6WMoBo7DGATtrMdhdXN5vtDyzBXwcmWRmJU"
  },
  {
    "vote_account": "4T799AaK9YT7zBtVqYZEnCY5ihUF5XaEYemwr5EnozoQ",
    "mev_revenue": "52154909",
    "claim_status_account": "7X8oKcJwtnfW4ZEdgSmeTGoV5j5QsBiwANLzRBMHuiow"
  },
  {
    "vote_account": "HxYHGzR58gyf6c4JAX85eK8GVuaZU2zne4be82Lq9SBQ",
    "mev_revenue": "73538613551",
    "claim_status_account": "2WaSuydoja2n7XAjdfhKTVG2tPUjUZHV6bazXiPymPNc"
  },
  {
    "vote_account": "QhyTEHb5JkMBki8Lq1npsaixefUyMXWJtbxK6jNjxnn",
    "mev_revenue": "9253398116",
    "claim_status_account": "HzadU97MtEHANXMbMWJxUk7EnteQbCMDXpqi7Wizvbup"
  },
  {
    "vote_account": "2uXzxR2EZVaHE3CuaDaUJ8C9Qr54LMfwkY5BtRPAzbPE",
    "mev_revenue": "145299847652",
    "claim_status_account": "2Pt39NH1iY7cnNit8QCY7Y1cW4X2du2zKdyAY3XMG5WQ"
  },
  {
    "vote_account": "8FPz3JG4E3HVXxGbPZVibarva4AGXSZWx3qKLUS5uFtN",
    "mev_revenue": "365365532",
    "claim_status_account": "EvVTvM41hS2cJrFcDoBWWCYfaVSRMoEcRCHqLFK1vES"
  },
  {
    "vote_account": "E9W5kU2fnha9yp4RmFZgNNsRUvy6oKnB9ZyR9LC81WaE",
    "mev_revenue": "87324573818",
    "claim_status_account": "7KSgZ7DjjodB8jrz7xjBEj6P1yBPvujhkfyoju2DdnKz"
  },
  {
    "vote_account": "DLKjd8DJc9NajCaHPeQL6BnhPi3a4BZm7zCdVF3MzDRZ",
    "mev_revenue": "20103893246",
    "claim_status_account": "3Qqq3dmJCQa3TLctMS8qSmxFGvYM9EeqDAL6HjB5GAvy"
  },
  {
    "vote_account": "nfGcSJkP35SkPa5475iBChmq1UNcj7JE1uQHrrasymm",
    "mev_revenue": "295764300",
    "claim_status_account": "J9gFeir5fdMawsRX9Z4W5xL7unbHx7xveFehnku9zNd7"
  },
  {
    "vote_account": "EcLPNfLFgCkbcTuvdeQ85pnQMgAfBDqi2dkoNVPrSyr5",
    "mev_revenue": "25223160817",
    "claim_status_account": "AwoMQ1UAJnbcTtaDW4tJmG7kBgW532YgGfp5Z8DNGxYZ"
  },
  {
    "vote_account": "GvZEwtCHZ7YtCkQCaLRVEXsyVvQkRDhJhQgB6akPme1e",
    "mev_revenue": "67810027114",
    "claim_status_account": "3b9v37tp4S1ZHSt9Y7VWAGehu2a6uKRXjbPRLEhE7HDQ"
  },
  {
    "vote_account": "7opSZGmevWhRDyLt5Wu38FZFjUyredGmMki4DNmxDnjd",
    "mev_revenue": "30564535998",
    "claim_status_account": "7G1K5rYMaBgXNNvKzXGzP6pUgkBPeUkW8ThLJAGXGW1b"
  },
  {
    "vote_account": "BkSS8kGUNcQkTgEKMmBhHxGVLdzw43EAzDYpZqyyxFrT",
    "mev_revenue": "44903653463",
    "claim_status_account": "8VqPqN2Moz5hcBqaXA6zULwCU6ETsPiwk7MLNkn7PzVV"
  },
  {
    "vote_account": "B38JgkTi7Fu2Uxk8JzNw4M7aMhVxzGu2fsRqHNScPkCQ",
    "mev_revenue": "477581040",
    "claim_status_account": "HHspyfQP6xbYx4whoscUGqnnsXwq3dq7F7FvHt8MqUBH"
  },
  {
    "vote_account": "7jPqpHuN5v59dtBom2tjmYEfi6WaM4sFtJeTD6fzhcdS",
    "mev_revenue": "29834344081",
    "claim_status_account": "A7h5oiBbA57pg4An5StsHWutt51LjLGMXPivqebqtqgs"
  },
  {
    "vote_account": "FjkSLYmi6BJAJQn1iSLUGrPrBQjMaD4y1DVdnv3yaTsX",
    "mev_revenue": "89617117020",
    "claim_status_account": "4HwUTBoYbniAbsj8u6TFuXiR4C6Bm1ZPQzdhCAaFcXJb"
  },
  {
    "vote_account": "privSGy4XbFCjzEkdLXssV8xMWRWWiWbJeDzh1emUyL",
    "mev_revenue": "158130988",
    "claim_status_account": "CXnM7DM1noWwxG6i7Hgknm5ssVmBSCrYEmRk789ouN3a"
  },
  {
    "vote_account": "6tgtejPHUHR1pECzXqQT8EHZqnKCWZFSqdZXDyBaKe3b",
    "mev_revenue": "8892361419",
    "claim_status_account": "23eBSUdU6eyoKw6bUeJXMg2VfguNPFB3c2hWtdAAXZ97"
  },
  {
    "vote_account": "Bkskrv38Kn7zJR5mvmbCDGn2M4Jyhzt2ZqwQXV6rYnXa",
    "mev_revenue": "27854297797",
    "claim_status_account": "G6n1akXkVZPA3XStUhzC1nQv4fz4PnFhRi218tRNHBn9"
  },
  {
    "vote_account": "6jzDwKeR21EFHwaRgZMefMxJ9D2vnQRqfYxkpUuJppPh",
    "mev_revenue": "105291710005",
    "claim_status_account": "DiZHUa8C5FZ2YmYzjJEf4isVMAiYVfzYPvpa37FaD6Yw"
  },
  {
    "vote_account": "HhYEE3dAShc3772wEiy73XDYnLjVxyBL8eAWKyRcF14y",
    "mev_revenue": "49361709279",
    "claim_status_account": "82Ta53hiRw9cw3wBT78BdoMJRtdkY6pG8rQjtDBy3fnq"
  },
  {
    "vote_account": "Bwkz1ddKoGE8hgiSV6HZLXi9RBLqfBi3HZb2QujzVGgz",
    "mev_revenue": "11222863579",
    "claim_status_account": "CEFsXx2HyZZUuHi5kvSTXZPQgaqKryiV5KShnhokeYbG"
  },
  {
    "vote_account": "irKsY8c3sQur1XaYuQ811hzsEQJ5Hq3Yu3AAoXYnp8W",
    "mev_revenue": "2907726052",
    "claim_status_account": "8FMotkb8QsPu3tZWz8kYLWsoYAspXEBVnGZaDnbJU4We"
  },
  {
    "vote_account": "H43AYFsvhNuALQieHpLXefp1ECgEBT6oVnS4EcTsC25C",
    "mev_revenue": "22768252024",
    "claim_status_account": "8ZtEi1xj9E1skx8BXrZmshwEtgqS9hPSUJwvKobe3gbP"
  },
  {
    "vote_account": "JDMq8hxZnad2smKLGkFbfg8zVMZHKQcMugD4tMR9u2da",
    "mev_revenue": "62569432561",
    "claim_status_account": "3mehuDsoL46Lm9qPdzrUwLqTg8Vqa9Jg5dVMMeDcHZ9t"
  },
  {
    "vote_account": "KRAKEnMdmT4EfM8ykTFH6yLoCd5vNLcQvJwF66Y2dag",
    "mev_revenue": "106470667323",
    "claim_status_account": "DefvzsrxT91PT3yTx24NH79ums75TdK99McsSm74UNEk"
  },
  {
    "vote_account": "2PEyBgsPYBQ8pMdXQtEaPGNqWQHE9GCnmV2tTVN4GMru",
    "mev_revenue": "1828135485",
    "claim_status_account": "BXqat7K5DeUJLPq3JDthFpXddHVbgaSG66BtDKP5YaSU"
  },
  {
    "vote_account": "8mHUDJjzPo2AwJp8SHKmG9rk9ftWTp7UysqYz36cMpJe",
    "mev_revenue": "73261265456",
    "claim_status_account": "3qodoHUDCxCbHoH4erSSyeP9WphXEShqikh2J6mkaYPf"
  },
  {
    "vote_account": "8Pep3GmYiijRALqrMKpez92cxvF4YPTzoZg83uXh14pW",
    "mev_revenue": "139550448919",
    "claim_status_account": "GJcPXBfQpNxdr5RQqr17KWKQbQJkRAmMq9jzrPhzd3p5"
  },
  {
    "vote_account": "8r4Fu6M8brgnL456RJfwxk8kN4iw1LgczfuXeuG1g4px",
    "mev_revenue": "15630565310",
    "claim_status_account": "2aonJfDRA24coeuF7BJYouFvCehEdn4wdTrXPKJTZ1dJ"
  },
  {
    "vote_account": "37BPVW1Ne1XHrzK15xguAS2BTdobVfThDzTE2mv8SsnJ",
    "mev_revenue": "88362923",
    "claim_status_account": "9fA2X9bw7NtuoTopV2sHYAkaaBFgXKwqMgfMLxvkn2D7"
  },
  {
    "vote_account": "8wTSPukwTAzNzEYyUdc8UiKkTg1hNtZ1xLum7o1Ne6wr",
    "mev_revenue": "70880724417",
    "claim_status_account": "71LGSGJahQAMobiXnUU7pg4TD3PSACcdM2JtSGGjyyxv"
  },
  {
    "vote_account": "iZADA4YKVRJZJaDUV3j79DzyK4VJkK3DGTfvvqvbC1K",
    "mev_revenue": "94165245061",
    "claim_status_account": "7bia3E5RW2mmPiLB5BPRvmfExhf2eQ4WGTZYCTM67aVQ"
  },
  {
    "vote_account": "6hTLQ5HSdWcpZkbXmZxXaGjCgTh7zh8UeWKWKgGE1BPp",
    "mev_revenue": "2758745646",
    "claim_status_account": "Eg38LEmpwtfGHSuq2YHMKFkqfDwVderETQy2aTzm3ofz"
  },
  {
    "vote_account": "EcEowA4GKDsdVBF9PNAZa6c9M4WgYG8y4GnpZSUaqioS",
    "mev_revenue": "2377297350",
    "claim_status_account": "CspzrdxTKbJre4asYaJzuDCW7DVQKfasjHA8SwZNJbgG"
  },
  {
    "vote_account": "EpRvips2doUUdxvs3Qhf4MCLqVeJEPu47Aci5QbBXASV",
    "mev_revenue": "3574725039",
    "claim_status_account": "3BKDzf6rPqaCa6k3BRGhFPT1UshNvJ1Njf1rtSVNEACE"
  },
  {
    "vote_account": "G9x1mqewTeVnXLmv3FamYD5tq1AdS395RHH3MLQPj6TY",
    "mev_revenue": "185307490928",
    "claim_status_account": "DUEPFSxZgB86qXBELaswtLeqiRXCWgVnaviwgEbbysh8"
  },
  {
    "vote_account": "EXhYxF25PJEHb3v5G1HY8Jn8Jm7bRjJtaxEghGrUuhQw",
    "mev_revenue": "96002013119",
    "claim_status_account": "JBKq1NS464ynehHHEZEo9Ek7YeuQgwebgEgc9M5uYqgz"
  },
  {
    "vote_account": "9QU2QSxhb24FUX3Tu2FpczXjpK3VYrvRudywSZaM29mF",
    "mev_revenue": "174736972763",
    "claim_status_account": "BvR2j1cahxkjdhwETS6JXhY8nUBUwc81FhDoo2fajXkz"
  },
  {
    "vote_account": "HZKopZYvv8v6un2H6KUNVQCnK5zM9emKKezvqhTBSpEc",
    "mev_revenue": "183433419305",
    "claim_status_account": "4bim5mFwMUNzbmWHVfD62b17x7daNZAFpYAWHf9C1DRF"
  },
  {
    "vote_account": "ADYZmUgm49MeEotqzz59eVtoeNKBv5d4jRn8xjvR2uj3",
    "mev_revenue": "1733291824",
    "claim_status_account": "5VsFpnjYRTrpdBpNg83CmDXudVdYwWhLxfxuBmVfAcuC"
  },
  {
    "vote_account": "At2rZHk554qWrjcmdNkCQGp8i4hdKLf52EXMrDmng5ab",
    "mev_revenue": "100530294060",
    "claim_status_account": "H6zEv4y1mKyacUDGLkvQNwkKEgrKdyNrmg7dpPNkxsyH"
  },
  {
    "vote_account": "33hurzEz6aEnzfESL6pnNyR6DCgcKzssT1pwSzDCBTRQ",
    "mev_revenue": "74289605308",
    "claim_status_account": "5HsV2FbNeNqz4Hb3X351NaaB5Aiq1cqgs2p1FDYfjDX5"
  },
  {
    "vote_account": "BU3ZgGBXFJwNTrN6VUJ88k9SJ71SyWfBJTabYqRErm4F",
    "mev_revenue": "62393193814",
    "claim_status_account": "94U9Jnvo4NfogN15SNhiwD8sFsUZUy3GyBCD7qwRNmKA"
  },
  {
    "vote_account": "Eajfs6oXGGkvjYsxkQZZJcDCLLkUajaHizfgg2xTsqyd",
    "mev_revenue": "8805216152",
    "claim_status_account": "E9n2RSeUzLWwQmZhYNHdkVqy1noAgo1b9j5JWo3JuAH1"
  },
  {
    "vote_account": "BLADE1qNA1uNjRgER6DtUFf7FU3c1TWLLdpPeEcKatZ2",
    "mev_revenue": "85974557984",
    "claim_status_account": "F8xLHVQ7tFXe2KMYx2UsLtvCNZYXx1dzrdyPzJyEKidk"
  },
  {
    "vote_account": "DQ7D6ZRtKbBSxCcAunEkoTzQhCBKLPdzTjPRRnM6wo1f",
    "mev_revenue": "45060084386",
    "claim_status_account": "fZgebAFmqitmy5Evve1WHwByjbtH7depFY6ibdAaJat"
  }
]  
            "#;

            account_analyzer::analyze_accounts_from_json(
                rpc_client,
                tip_distribution_program_id,
                tip_router_program_id,
                ncn_address,
                epoch,
                &json_808_data,
            )
            .await?;
        }
    }
    Ok(())
}

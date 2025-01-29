use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::EllipsisClient,
    log::{error, info},
    meta_merkle_tree::generated_merkle_tree::{GeneratedMerkleTreeCollection, StakeMetaCollection},
    solana_metrics::{datapoint_error, datapoint_info, set_host_id},
    solana_rpc_client::rpc_client::RpcClient,
    solana_runtime::bank::Bank,
    solana_sdk::{pubkey::Pubkey, signer::keypair::read_keypair_file},
    std::{
        path::PathBuf,
        str::FromStr,
        sync::Arc,
        time::{Duration, Instant},
    },
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
        claim::claim_mev_tips,
        cli::{Cli, Commands},
        create_merkle_tree_collection, create_meta_merkle_tree, create_stake_meta,
        ledger_utils::get_bank_from_ledger,
        process_epoch::{get_previous_epoch_last_slot, wait_for_next_epoch},
        submit::{submit_recent_epochs_to_ncn, submit_to_ncn},
        OperatorState,
    },
    tokio::time::sleep,
};

// TODO: Should this be loaded from somewhere?
const PROTOCOL_FEE_BPS: u64 = 300;

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

    info!(
        "CLI Arguments:
        keypair_path: {}
        operator_address: {}
        rpc_url: {}
        ledger_path: {}
        account_paths: {:?}
        full_snapshots_path: {:?}
        snapshot_output_dir: {}",
        cli.keypair_path,
        cli.operator_address,
        cli.rpc_url,
        cli.ledger_path.display(),
        cli.account_paths,
        cli.full_snapshots_path,
        cli.snapshot_output_dir.display()
    );

    match cli.command {
        Commands::Run {
            ncn_address,
            tip_distribution_program_id,
            tip_payment_program_id,
            tip_router_program_id,
            enable_snapshots,
            num_monitored_epochs,
            override_target_slot,
            starting_stage,
            save_stages,
        } => {
            info!("Running Tip Router...");
            info!("starting stage: {:?}", starting_stage);

            let operator_address = cli.operator_address.clone();
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

            // Track runs that are starting right at the beginning of a new epoch
            let mut stage = starting_stage;
            let mut bank: Option<Arc<Bank>> = None;
            let mut stake_meta_collection: Option<StakeMetaCollection> = None;
            let mut merkle_tree_collection: Option<GeneratedMerkleTreeCollection> = None;
            let account_paths = cli
                .account_paths
                .map_or_else(|| vec![cli.ledger_path.clone()], |paths| paths);
            let mut slot_to_process = if let Some(slot) = override_target_slot {
                slot
            } else {
                0
            };
            let mut epoch_to_process = 0;
            loop {
                match stage {
                    OperatorState::LoadBankFromSnapshot => {
                        bank = Some(get_bank_from_ledger(
                            operator_address.clone(),
                            &cli.ledger_path,
                            account_paths.clone(),
                            cli.full_snapshots_path.clone().unwrap(),
                            cli.backup_snapshots_dir.clone(),
                            &slot_to_process,
                            enable_snapshots,
                        ));
                        // Transition to the next stage
                        stage = OperatorState::CreateStakeMeta;
                    }
                    OperatorState::CreateStakeMeta => {
                        // TODO: Determine if we want to allow operators to start from this stage.
                        //  No matter what a bank has to be loaded from a snapshot, so might as
                        //  well start from load bank
                        stake_meta_collection = Some(create_stake_meta(
                            operator_address.clone(),
                            epoch_to_process,
                            bank.as_ref().expect("Bank was not set"),
                            &tip_distribution_program_id,
                            &tip_payment_program_id,
                            &cli.save_path,
                            save_stages,
                        ));
                        // we should be able to safely drop the bank in this loop
                        bank = None;
                        // Transition to the next stage
                        stage = OperatorState::CreateMerkleTreeCollection;
                    }
                    OperatorState::CreateMerkleTreeCollection => {
                        let some_stake_meta_collection = match stake_meta_collection.to_owned() {
                            Some(collection) => collection,
                            // TODO: Handle this
                            None => todo!("load stake meta from disk given desired epoch"),
                        };

                        // Generate the merkle tree collection
                        merkle_tree_collection = Some(create_merkle_tree_collection(
                            cli.operator_address.clone(),
                            some_stake_meta_collection,
                            epoch_to_process,
                            &ncn_address,
                            PROTOCOL_FEE_BPS,
                            &cli.save_path,
                            save_stages,
                        ));

                        stake_meta_collection = None;
                        // Transition to the next stage
                        stage = OperatorState::CreateMetaMerkleTree;
                    }
                    OperatorState::CreateMetaMerkleTree => {
                        let some_merkle_tree_collection = match merkle_tree_collection.to_owned() {
                            Some(collection) => collection,
                            None => {
                                // TODO: Handle this
                                todo!("load merkle tree collection from disk given desired epoch")
                            }
                        };

                        create_meta_merkle_tree(
                            cli.operator_address.clone(),
                            some_merkle_tree_collection,
                            epoch_to_process,
                            &cli.save_path,
                            // TODO: If we keep the separate thread for handling NCN submission
                            //  through files on disk then this needs to be true
                            save_stages,
                        );
                        stage = OperatorState::WaitForNextEpoch;
                    }
                    OperatorState::SubmitToNcn => {
                        // TODO: Determine if this should be a stage given the task that's in a
                        //  separate thread
                    }
                    OperatorState::WaitForNextEpoch => {
                        wait_for_next_epoch(&rpc_client).await?;
                        // Get the last slot of the previous epoch
                        let (previous_epoch, previous_epoch_slot) =
                            if let Ok((epoch, slot)) = get_previous_epoch_last_slot(&rpc_client) {
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
        Commands::SnapshotSlot { slot } => {
            info!("Snapshotting slot...");
            let account_paths = cli
                .account_paths
                .map_or_else(|| vec![cli.ledger_path.clone()], |paths| paths);

            get_bank_from_ledger(
                cli.operator_address,
                &cli.ledger_path,
                account_paths,
                cli.full_snapshots_path.unwrap(),
                cli.backup_snapshots_dir,
                &slot,
                true,
            );
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
        Commands::ClaimTips {
            tip_distribution_program_id,
            micro_lamports,
            epoch,
        } => {
            let start = Instant::now();
            info!("Claiming tips...");

            let arc_keypair = Arc::new(keypair);
            // Load the GeneratedMerkleTreeCollection, which should have been previously generated
            let merkle_tree_coll_path = PathBuf::from(format!(
                "{}/merkle_tree_coll_{}.json",
                cli.meta_merkle_tree_dir.display(),
                epoch
            ));
            let merkle_tree_coll =
                GeneratedMerkleTreeCollection::new_from_file(&merkle_tree_coll_path)?;
            match claim_mev_tips(
                &merkle_tree_coll,
                cli.rpc_url.clone(),
                // TODO: Review if we should offer separate rpc_send_url in CLI. This may be used
                //  if sending via block engine.
                cli.rpc_url,
                tip_distribution_program_id,
                arc_keypair,
                Duration::from_secs(3600),
                micro_lamports,
            )
            .await
            {
                Err(e) => {
                    datapoint_error!(
                        "claim_mev_workflow-claim_error",
                        ("epoch", epoch, i64),
                        ("error", 1, i64),
                        ("err_str", e.to_string(), String),
                        ("elapsed_us", start.elapsed().as_micros(), i64),
                    );
                }
                Ok(()) => {
                    datapoint_info!(
                        "claim_mev_workflow-claim_completion",
                        ("epoch", epoch, i64),
                        ("elapsed_us", start.elapsed().as_micros(), i64),
                    );
                }
            }
        }
        Commands::CreateStakeMeta {
            epoch,
            slot,
            tip_distribution_program_id,
            tip_payment_program_id,
            save,
        } => {
            let account_paths = cli
                .account_paths
                .map_or_else(|| vec![cli.ledger_path.clone()], |paths| paths);
            let bank = get_bank_from_ledger(
                cli.operator_address.clone(),
                &cli.ledger_path,
                account_paths,
                cli.full_snapshots_path.unwrap(),
                cli.backup_snapshots_dir,
                &slot,
                true,
            );

            create_stake_meta(
                cli.operator_address,
                epoch,
                &bank,
                &tip_distribution_program_id,
                &tip_payment_program_id,
                &cli.save_path,
                save,
            );
        }
        Commands::CreateMerkleTreeCollection {
            ncn_address,
            epoch,
            save,
        } => {
            // Load the stake_meta_collection from disk
            let stake_meta_collection = match StakeMetaCollection::new_from_file(&cli.save_path) {
                Ok(stake_meta_collection) => stake_meta_collection,
                Err(e) => panic!("{}", e), // TODO: should datapoint error be emitted here?
            };

            // Generate the merkle tree collection
            create_merkle_tree_collection(
                cli.operator_address,
                stake_meta_collection,
                epoch,
                &ncn_address,
                PROTOCOL_FEE_BPS,
                &cli.save_path,
                save,
            );
        }
        Commands::CreateMetaMerkleTree { epoch, save } => {
            // Load the stake_meta_collection from disk
            let merkle_tree_collection =
                match GeneratedMerkleTreeCollection::new_from_file(&cli.save_path) {
                    Ok(merkle_tree_collection) => merkle_tree_collection,
                    Err(e) => panic!("{}", e), // TODO: should datapoint error be emitted here?
                };

            create_meta_merkle_tree(
                cli.operator_address,
                merkle_tree_collection,
                epoch,
                &cli.save_path,
                save,
            );
        }
    }
    Ok(())
}

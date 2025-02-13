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
    std::{path::PathBuf, str::FromStr, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        backup_snapshots::BackupSnapshotMonitor,
        claim::claim_mev_tips_with_emit,
        cli::{Cli, Commands},
        create_merkle_tree_collection, create_meta_merkle_tree, create_stake_meta,
        ledger_utils::get_bank_from_ledger,
        load_bank_from_snapshot, meta_merkle_tree_file_name, process_epoch,
        submit::{submit_recent_epochs_to_ncn, submit_to_ncn},
        PROTOCOL_FEE_BPS,
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

    cli.create_save_path();

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
            set_merkle_roots,
            claim_tips,
        } => {
            info!("Running Tip Router...");
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
                        let epoch = if let Ok(epoch) = rpc_client.get_epoch_info().await {
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
                enable_snapshots,
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
            let meta_merkle_tree_path = PathBuf::from(format!(
                "{}/{}",
                cli.save_path.display(),
                meta_merkle_tree_file_name(epoch)
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
        Commands::CreateStakeMeta {
            epoch,
            slot,
            tip_distribution_program_id,
            tip_payment_program_id,
            save,
        } => {
            let account_paths = vec![cli.ledger_path.clone()];
            let bank = get_bank_from_ledger(
                cli.operator_address.clone(),
                &cli.ledger_path,
                account_paths,
                cli.full_snapshots_path.unwrap(),
                cli.backup_snapshots_dir,
                &slot,
                false,
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
            tip_router_program_id,
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
                &tip_router_program_id,
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

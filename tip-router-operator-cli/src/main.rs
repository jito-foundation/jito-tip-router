use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::{ClientSubset, EllipsisClient},
    log::{error, info},
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        clock::DEFAULT_SLOTS_PER_EPOCH,
        signer::{keypair::read_keypair_file, Signer},
        transaction::Transaction,
    },
    std::time::Duration,
    tip_router_operator_cli::{
        cli::{Cli, Commands},
        process_epoch::{get_previous_epoch_last_slot, process_epoch, wait_for_next_epoch},
        submit::submit_recent_epochs_to_ncn,
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

    let test_meta_merkle_root = [1; 32];
    let ix = spl_memo::build_memo(&test_meta_merkle_root.to_vec(), &[&keypair.pubkey()]);
    info!("Submitting test memo {:?}", test_meta_merkle_root);

    let tx = Transaction::new_with_payer(&[ix], Some(&keypair.pubkey()));
    rpc_client.process_transaction(tx, &[&keypair]).await?;

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
        } => {
            info!("Running Tip Router...");

            // TODO turn into arc
            let rpc_client_clone = rpc_client.clone();
            let cli_clone = cli.clone();

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
                    sleep(Duration::from_secs(60)).await;
                }
            });

            loop {
                // Get the last slot of the previous epoch
                let (previous_epoch, previous_epoch_slot) =
                    get_previous_epoch_last_slot(&rpc_client).await?;
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
                    &cli,
                )
                .await
                {
                    Ok(_) => info!("Successfully processed epoch"),
                    Err(e) => {
                        error!("Error processing epoch: {}", e);
                        // Continue to next epoch even if this one failed
                    }
                }

                // Wait for epoch change
                wait_for_next_epoch(&rpc_client).await?;
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
                &cli,
            )
            .await
            {
                Ok(_) => info!("Successfully processed slot"),
                Err(e) => {
                    error!("Error processing epoch: {}", e);
                    // Continue to next epoch even if this one failed
                }
            }
        }
    }
    Ok(())
}

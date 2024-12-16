use ::{
    anyhow::Result,
    clap::Parser,
    ellipsis_client::EllipsisClient,
    log::{error, info},
    meta_merkle_tree::{
        generated_merkle_tree::{
            GeneratedMerkleTreeCollection as MetaMerkleTreeCollection,
            StakeMetaCollection as MetaMerkleStakeMetaCollection,
        },
        meta_merkle_tree::MetaMerkleTree,
    },
    solana_metrics::datapoint_info,
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        pubkey::Pubkey,
        signer::keypair::{read_keypair_file, Keypair},
    },
    std::{fs::File, io::BufReader, path::PathBuf, sync::Arc, time::Duration},
    tip_router_operator_cli::{
        claim_mev_workflow,
        cli::{Cli, Commands},
        merkle_root_generator_workflow, merkle_root_upload_workflow,
        process_epoch::{get_previous_epoch_last_slot, process_epoch, wait_for_next_epoch},
        snapshot::SnapshotCreator,
        stake_meta_generator_workflow, GeneratedMerkleTreeCollection, StakeMetaCollection,
    },
    tokio::time::Instant,
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

    match &cli.command {
        Commands::Monitor {
            ncn_address,
            tip_distribution_program_id,
            tip_payment_program_id,
        } => {
            info!("Starting epoch monitor...");

            loop {
                // Wait for epoch change
                wait_for_next_epoch(&rpc_client).await?;

                // Get the last slot of the previous epoch
                let previous_epoch_slot = get_previous_epoch_last_slot(&rpc_client).await?;
                info!("Processing slot {} for previous epoch", previous_epoch_slot);

                let snapshot_creator = SnapshotCreator::new(
                    &cli.rpc_url,
                    cli.snapshot_output_dir.to_str().unwrap().to_string(),
                    5,
                    "zstd".to_string(),
                    Keypair::from_bytes(&keypair.to_bytes()).unwrap(),
                    cli.ledger_path.clone(),
                )?;

                // Process the epoch
                match process_epoch(
                    previous_epoch_slot,
                    &cli,
                    &keypair,
                    snapshot_creator,
                    tip_distribution_program_id,
                    tip_payment_program_id,
                    ncn_address,
                )
                .await
                {
                    Ok(_) => info!("Successfully processed epoch"),
                    Err(e) => {
                        error!("Error processing epoch: {}", e);
                        // Continue to next epoch even if this one failed
                    }
                }
            }
        }
    }
}

use {
    anyhow::Result,
    clap::Parser,
    log::info,
    solana_client::rpc_client::RpcClient,
    solana_metrics::datapoint_info,
    solana_sdk::{
        pubkey::Pubkey,
        signer::{
            keypair::{read_keypair_file, Keypair},
            Signer,  // Add this trait import
        },
        slot_history::Slot,
    },
    std::path::PathBuf,
    tip_router_operator_cli::{
        snapshot::SnapshotCreator,  // Import SnapshotCreator through the crate path
        *,
    },
};

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    /// Path to operator keypair
    #[arg(short, long)]
    keypair_path: String,

    /// RPC URL
    #[arg(short, long, default_value = "http://localhost:8899")]
    rpc_url: String,

    /// Path to ledger
    #[arg(short, long)]
    ledger_path: PathBuf,

    /// Snapshot output directory
    #[arg(short, long)]
    snapshot_output_dir: PathBuf,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    Monitor {
        #[arg(short, long)]
        ncn_address: String,

        /// Tip distribution program ID
        #[arg(long)]
        tip_distribution_program_id: Pubkey,

        /// Tip payment program ID
        #[arg(long)]
        tip_payment_program_id: Pubkey,
    },
}

async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<Slot> {
    let epoch_info = rpc_client.get_epoch_info()?;
    // let current_epoch: u64 = epoch_info.epoch;
    let current_slot = epoch_info.absolute_slot;
    let slot_index = epoch_info.slot_index;

    let epoch_start_slot = current_slot - slot_index;
    let previous_epoch_final_slot = epoch_start_slot - 1;

    // Find the last slot with a block
    let mut slot = previous_epoch_final_slot;
    while rpc_client.get_block(slot).is_err() {
        slot -= 1;
    }

    Ok(slot)
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let rpc_client = RpcClient::new(cli.rpc_url.clone());

    match cli.command {
        Commands::Monitor { ncn_address, tip_distribution_program_id, tip_payment_program_id } => {
            loop {
                let previous_epoch_slot = get_previous_epoch_last_slot(&rpc_client).await?;
                info!("Processing slot {} for previous epoch", previous_epoch_slot);

                // 1. Create snapshot
                let snapshot_creator = SnapshotCreator::new(
                    &cli.rpc_url,
                    cli.snapshot_output_dir.to_str().unwrap().to_string(),
                    5, // max_snapshots
                    "bzip2".to_string(),
                    read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
                    cli.ledger_path.clone()
                )?;

                snapshot_creator.create_snapshot(previous_epoch_slot).await?;

                // 2. Generate stake metadata
                let stake_meta_path = cli.snapshot_output_dir.join(
                    format!("stake-meta-{}.json", previous_epoch_slot)
                );

                let merkle_root_upload_authority = Keypair::new();

                let merkle_tree_generator = {
                    let _merkle_root_upload_authority = Keypair::new();
                    let authority_pubkey = _merkle_root_upload_authority.pubkey();  // Get pubkey before moving
                    merkle_tree::MerkleTreeGenerator::new(
                        &cli.rpc_url,
                        read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
                        ncn_address,
                        cli.snapshot_output_dir.clone(),
                        tip_distribution_program_id,
                        merkle_root_upload_authority,  // Move happens here
                        authority_pubkey,  // Use the previously obtained pubkey
                    )?
                };

                stake_meta_generator_workflow::generate_stake_meta(
                    &cli.ledger_path,
                    &previous_epoch_slot,
                    &tip_distribution_program_id,
                    stake_meta_path.to_str().unwrap(),
                    &tip_payment_program_id
                )?;

                // Load the stake metadata from the generated file
                let file = std::fs::File::open(&stake_meta_path)?;
                let stake_meta_collection: StakeMetaCollection = serde_json::from_reader(file)?;

                // 3. Create merkle trees
                let merkle_trees =
                    merkle_tree_generator.generate_and_upload_merkle_trees(
                        stake_meta_collection
                    ).await?;

                datapoint_info!(
                    "tip_router_merkle_trees",
                    ("count", merkle_trees.generated_merkle_trees.len(), i64),
                    ("slot", previous_epoch_slot, i64)
                );

                // 4. Create meta merkle tree
                let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(
                    &merkle_trees
                ).await?;

                // 5. Upload meta merkle tree to NCN
                merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

                info!("Generated and uploaded merkle trees and meta merkle tree for epoch");

                // Wait for next epoch
                merkle_tree_generator.wait_for_epoch_boundary().await?;

                break Ok(());
            }
        }
    }
}

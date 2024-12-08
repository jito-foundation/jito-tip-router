use ::{
    anyhow::Result,
    clap::Parser,
    log::{ info, error },
    solana_client::rpc_client::RpcClient,
    solana_sdk::{ pubkey::Pubkey, signature::Keypair, signer::keypair::read_keypair_file },
    std::{ sync::Arc, time::Duration, path::PathBuf },
    tip_router_operator_cli::{
        merkle_tree::MerkleTreeGenerator,
        claim_mev_workflow,
        merkle_root_generator_workflow,
        stake_meta_generator_workflow,
        snapshot::SnapshotCreator,
        StakeMetaCollection, // Add this import
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
        ncn_address: Pubkey,

        /// Tip distribution program ID
        #[arg(long)]
        tip_distribution_program_id: Pubkey,

        /// Tip payment program ID
        #[arg(long)]
        tip_payment_program_id: Pubkey,
    },
}

async fn wait_for_next_epoch(rpc_client: &RpcClient) -> Result<()> {
    let current_epoch = rpc_client.get_epoch_info()?.epoch;

    loop {
        tokio::time::sleep(Duration::from_secs(10)).await; // Check every 10 seconds
        let new_epoch = rpc_client.get_epoch_info()?.epoch;

        if new_epoch > current_epoch {
            info!("New epoch detected: {} -> {}", current_epoch, new_epoch);
            return Ok(());
        }
    }
}

async fn get_previous_epoch_last_slot(rpc_client: &RpcClient) -> Result<u64> {
    let epoch_info = rpc_client.get_epoch_info()?;
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

async fn process_epoch(
    previous_epoch_slot: u64,
    cli: &Cli,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey
) -> Result<()> {
    // 1. Create snapshot
    info!("1. Creating snapshot of previous epoch...");
    let snapshot_creator = SnapshotCreator::new(
        &cli.rpc_url,
        cli.snapshot_output_dir.to_str().unwrap().to_string(),
        5,
        "bzip2".to_string(),
        read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
        cli.ledger_path.clone()
    )?;
    snapshot_creator.create_snapshot(previous_epoch_slot).await?;

    // 2. Generate stake metadata
    info!("2. Generating stake metadata from snapshot...");
    let stake_meta_path = cli.snapshot_output_dir.join("stake-meta.json");

    // First generate the stake meta file
    stake_meta_generator_workflow::generate_stake_meta(
        &cli.ledger_path,
        &previous_epoch_slot,
        tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        tip_payment_program_id
    )?;

    // Then read the generated file to get the StakeMetaCollection
    let stake_meta: StakeMetaCollection = serde_json::from_reader(
        std::fs::File::open(&stake_meta_path)?
    )?;

    info!("Successfully loaded stake meta collection from generated file");

    // 3. Create merkle trees
    info!("3. Generating merkle trees for each validator...");
    let merkle_tree_path = cli.snapshot_output_dir.join("merkle-trees");
    merkle_root_generator_workflow::generate_merkle_root(
        &stake_meta_path,
        &merkle_tree_path,
        &cli.rpc_url
    )?;

    // 4. Initialize MerkleTreeGenerator for meta tree and uploads
    info!("4. Initializing MerkleTreeGenerator...");
    let merkle_tree_generator = MerkleTreeGenerator::new(
        &cli.rpc_url,
        read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
        *ncn_address,
        merkle_tree_path.clone(),
        *tip_distribution_program_id,
        Keypair::new(),
        Pubkey::new_unique()
    )?;

    // 5. Generate and upload individual merkle trees
    info!("5. Generating and uploading merkle trees...");
    let merkle_trees = merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta).await?;

    // 6. Generate meta merkle tree
    info!("6. Generating meta merkle tree...");
    let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;

    info!("Generated meta merkle tree: {:?}", meta_merkle_tree);

    // 7. Upload meta merkle root to NCN
    info!("7. Uploading meta merkle root to NCN...");
    merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

    // 8. Optional: Test claiming
    info!("8. Testing tip claiming capability...");
    let context_keypair = read_keypair_file(&cli.keypair_path).expect(
        "Failed to read keypair file"
    );

    claim_mev_workflow::claim_mev_tips(
        &merkle_trees,
        cli.rpc_url.clone(),
        *tip_distribution_program_id,
        Arc::new(context_keypair),
        Duration::from_secs(10),
        1
    ).await?;

    info!("Successfully completed all steps for epoch processing");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let rpc_client = RpcClient::new(cli.rpc_url.clone());

    match &cli.command {
        Commands::Monitor { ncn_address, tip_distribution_program_id, tip_payment_program_id } => {
            info!("Starting epoch monitor...");

            loop {
                // Wait for epoch change
                wait_for_next_epoch(&rpc_client).await?;

                // Get the last slot of the previous epoch
                let previous_epoch_slot = get_previous_epoch_last_slot(&rpc_client).await?;
                info!("Processing slot {} for previous epoch", previous_epoch_slot);

                // Process the epoch
                match
                    process_epoch(
                        previous_epoch_slot,
                        &cli,
                        tip_distribution_program_id,
                        tip_payment_program_id,
                        ncn_address
                    ).await
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

use ::{
    anyhow::Result,
    clap::Parser,
    log::{ info, error },
    solana_rpc_client::rpc_client::RpcClient,
    solana_sdk::{
        pubkey::Pubkey,
        signer::keypair::read_keypair_file,
    },
    std::{ 
        sync::Arc, 
        time::Duration, 
        path::PathBuf,
        fs::File,
        io::BufReader,
    },
    tip_router_operator_cli::{
        claim_mev_workflow,
        merkle_root_generator_workflow,
        merkle_root_upload_workflow,
        stake_meta_generator_workflow,
        snapshot::SnapshotCreator,
        StakeMetaCollection,
        GeneratedMerkleTreeCollection,
    },
    tokio::time::Instant,
    ellipsis_client::EllipsisClient,
    solana_metrics::datapoint_info,
    meta_merkle_tree::{
        meta_merkle_tree::MetaMerkleTree,
        generated_merkle_tree::GeneratedMerkleTreeCollection as MetaMerkleTreeCollection,
        generated_merkle_tree::StakeMetaCollection as MetaMerkleStakeMetaCollection,
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

async fn wait_for_next_epoch(rpc_client: &EllipsisClient) -> Result<()> {
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

async fn get_previous_epoch_last_slot(rpc_client: &EllipsisClient) -> Result<u64> {
    let epoch_info = rpc_client.get_epoch_info()?;
    let current_slot = epoch_info.absolute_slot;
    let slot_index = epoch_info.slot_index;

    // Handle case where we're in the first epoch
    if current_slot < slot_index {
        return Ok(0);
    }

    let epoch_start_slot = current_slot - slot_index;
    let previous_epoch_final_slot = epoch_start_slot.saturating_sub(1);

    Ok(previous_epoch_final_slot)
}

async fn process_epoch(
    previous_epoch_slot: u64,
    cli: &Cli,
    client: &EllipsisClient,
    tip_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
    ncn_address: &Pubkey
) -> Result<()> {
    let process_start = Instant::now();

    // 1. Create snapshot
    info!("1. Creating snapshot of previous epoch...");
    let snapshot_start = Instant::now();
    let snapshot_creator = SnapshotCreator::new(
        &cli.rpc_url,
        cli.snapshot_output_dir.to_str().unwrap().to_string(),
        5,
        "bzip2".to_string(),
        read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file"),
        cli.ledger_path.clone()
    )?;
    let snapshot_result = snapshot_creator.create_snapshot(previous_epoch_slot).await;
    datapoint_info!(
        "tip_router_snapshot",
        ("success", snapshot_result.is_ok(), bool),
        ("duration_ms", snapshot_start.elapsed().as_millis() as i64, i64),
        ("epoch_slot", previous_epoch_slot as i64, i64)
    );
    snapshot_result?;

    // 2. Generate stake metadata
    info!("2. Generating stake metadata from snapshot...");
    let stake_meta_start = Instant::now();
    let stake_meta_path = cli.snapshot_output_dir.join("stake-meta.json");
    let stake_meta_result = stake_meta_generator_workflow::generate_stake_meta(
        &cli.ledger_path,
        &previous_epoch_slot,
        tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        tip_payment_program_id
    );
    datapoint_info!(
        "tip_router_stake_meta",
        ("success", stake_meta_result.is_ok(), bool),
        ("duration_ms", stake_meta_start.elapsed().as_millis() as i64, i64)
    );
    stake_meta_result?;

    // Load stake meta
    let stake_meta: StakeMetaCollection = serde_json::from_reader(
        std::fs::File::open(&stake_meta_path)?
    )?;
    datapoint_info!(
        "tip_router_stake_meta_size",
        ("stake_meta_count", stake_meta.stake_metas.len() as i64, i64)
    );

    // 3. Create merkle trees
    info!("3. Generating merkle trees for each validator...");
    let merkle_gen_start = Instant::now();
    let merkle_tree_path = cli.snapshot_output_dir.join("merkle-trees");
    let merkle_gen_result = merkle_root_generator_workflow::generate_merkle_root(
        &stake_meta_path,
        &merkle_tree_path,
        client
    );
    datapoint_info!(
        "tip_router_merkle_generation",
        ("success", merkle_gen_result.is_ok(), bool),
        ("duration_ms", merkle_gen_start.elapsed().as_millis() as i64, i64)
    );
    merkle_gen_result?;

    // 4. Upload merkle roots
    info!("4. Uploading merkle trees...");
    let upload_start = Instant::now();
    merkle_root_upload_workflow::upload_merkle_root(
        &merkle_tree_path,
        &PathBuf::from(&cli.keypair_path),
        client,
        tip_distribution_program_id,
        5,
        10
    ).await?;

     // 5. Generate meta merkle tree
     info!("5. Generating meta merkle tree...");
     let meta_start = Instant::now();
     
     // Load the generated merkle trees directly
     let file = File::open(&merkle_tree_path)?;
     let reader = BufReader::new(file);
     let meta_merkle_trees: MetaMerkleTreeCollection = serde_json::from_reader(reader)?;
     
     // Create meta merkle tree
     let meta_merkle_tree = MetaMerkleTree::new_from_generated_merkle_tree_collection(
         meta_merkle_trees.clone()
     )?;
 
     // Save meta merkle tree
     let meta_merkle_path = cli.snapshot_output_dir.join("meta-merkle-tree.json");
     meta_merkle_tree.write_to_file(&meta_merkle_path);
 
     // Convert for claim testing
     let generated_trees: GeneratedMerkleTreeCollection = serde_json::from_str(
         &serde_json::to_string(&meta_merkle_trees)?
     )?;

    datapoint_info!(
        "tip_router_meta_merkle",
        ("duration_ms", meta_start.elapsed().as_millis() as i64, i64),
        ("num_nodes", meta_merkle_tree.num_nodes as i64, i64)
    );

    // Need to implement upload to NCN

    // 6. Test claiming
    info!("6. Testing tip claiming capability...");
    let claim_start = Instant::now();
    let context_keypair = read_keypair_file(&cli.keypair_path).expect(
    "Failed to read keypair file"
    );

    // Load the generated merkle trees directly from file
    let file = File::open(&merkle_tree_path)?;
    let reader = BufReader::new(file);
    let generated_trees: GeneratedMerkleTreeCollection = serde_json::from_reader(reader)?;

    let claim_result = claim_mev_workflow::claim_mev_tips(
        &generated_trees,
        client,
        *tip_distribution_program_id,
        Arc::new(context_keypair),
        Duration::from_secs(10),
        1
    ).await;

    datapoint_info!(
        "tip_router_claim_test",
        ("success", claim_result.is_ok(), bool),
        ("duration_ms", claim_start.elapsed().as_millis() as i64, i64)
    );

    // Overall process metrics
    datapoint_info!(
        "tip_router_epoch_process",
        ("total_duration_ms", process_start.elapsed().as_millis() as i64, i64),
        ("epoch_slot", previous_epoch_slot as i64, i64),
        ("success", true, bool)
    );

    info!("Successfully completed all steps for epoch processing");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let rpc_client = EllipsisClient::from_rpc(
        RpcClient::new(cli.rpc_url.clone()),
        &read_keypair_file(&cli.keypair_path).expect("Failed to read keypair file")
    )?;

    match &cli.command {
        Commands::Monitor { ncn_address, tip_distribution_program_id, tip_payment_program_id } => {
            info!("Starting epoch monitor...");

            loop {
                wait_for_next_epoch(&rpc_client).await?;
                let previous_epoch_slot = get_previous_epoch_last_slot(&rpc_client).await?;
                info!("Processing slot {} for previous epoch", previous_epoch_slot);

                match process_epoch(
                    previous_epoch_slot,
                    &cli,
                    &rpc_client,
                    tip_distribution_program_id,
                    tip_payment_program_id,
                    ncn_address
                ).await {
                    Ok(_) => info!("Successfully processed epoch"),
                    Err(e) => {
                        error!("Error processing epoch: {}", e);
                    }
                }
            }
        }
    }
}

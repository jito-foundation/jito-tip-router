use {
    solana_sdk::{ signer::keypair::Keypair, pubkey::Pubkey, slot_history::Slot },
    solana_runtime::{
        bank::Bank,
        genesis_utils::{
            create_genesis_config_with_vote_accounts,
            ValidatorVoteKeypairs,
            GenesisConfigInfo,
        },
        runtime_config::RuntimeConfig,
    },
    tip_router_operator_cli::{
        merkle_tree::MerkleTreeGenerator,
        stake_meta_generator_workflow,
        StakeMetaCollection,
    },
    std::{ path::Path, fs, sync::Arc },
    tempfile::TempDir,
    anyhow::{ Result, anyhow },
    solana_ledger::{
        bank_forks_utils,
        blockstore::Blockstore,
        blockstore_processor::ProcessOptions,
        blockstore_options::{ AccessType, BlockstoreOptions, LedgerColumnOptions },
    },
    solana_accounts_db::{
        hardened_unpack::open_genesis_config,
        accounts_index::AccountSecondaryIndexes,
        accounts_db::AccountShrinkThreshold,
    },
    std::sync::atomic::AtomicBool,
    solana_sdk::genesis_config::GenesisConfig,
    std::io::Write,
};

fn create_genesis_files(ledger_dir: &Path, genesis_config: &GenesisConfig) -> Result<()> {
    let genesis_path = ledger_dir.join("genesis.bin");
    println!("\nPreparing genesis.bin at: {:?}", genesis_path);
    
    // Force remove any existing file/directory at the path
    if genesis_path.exists() {
        if genesis_path.is_dir() {
            fs::remove_dir_all(&genesis_path)?;
        } else {
            fs::remove_file(&genesis_path)?;
        }
    }

    // Create the file first with explicit options
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&genesis_path)?;

    // Serialize genesis config to bytes and write directly to file
    let serialized = bincode::serialize(&genesis_config)?;
    file.write_all(&serialized)?;
    file.sync_all()?;

    // Create tar file
    let genesis_tar_path = ledger_dir.join("genesis.tar.bz2");
    let mut tar_buffer = Vec::new();
    {
        let encoder = bzip2::write::BzEncoder::new(&mut tar_buffer, bzip2::Compression::best());
        let mut tar = tar::Builder::new(encoder);
        let mut header = tar::Header::new_gnu();
        header.set_path("genesis.bin")?;
        header.set_size(serialized.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        tar.append(&header, &serialized[..])?;
        let encoder = tar.into_inner()?;
        encoder.finish()?;
    }

    fs::write(&genesis_tar_path, &tar_buffer)?;

    Ok(())
}

fn create_bank_from_snapshot(ledger_path: &Path, snapshot_slot: &Slot) -> Result<Arc<Bank>> {
    println!("Ledger path: {:?}", ledger_path);
    println!("Checking if ledger path exists: {}", ledger_path.exists());

    // Check for genesis file
    let genesis_path = ledger_path.join("genesis.bin");
    println!("Genesis path: {:?}", genesis_path);
    println!("Checking if genesis file exists: {}", genesis_path.exists());
    println!("Is genesis path a file? {}", genesis_path.is_file());
    println!("Is genesis path a directory? {}", genesis_path.is_dir());

    // Try to read the genesis file contents
    println!("Attempting to read genesis file...");
    let genesis_contents = fs::read(&genesis_path)?;
    println!("Genesis file size: {} bytes", genesis_contents.len());

    println!("Attempting to open genesis config...");
    let genesis_config = open_genesis_config(ledger_path, 10_485_760)?;
    println!("Genesis config opened successfully");

    let blockstore = Blockstore::open_with_options(ledger_path, BlockstoreOptions {
        access_type: AccessType::Secondary,
        recovery_mode: None,
        enforce_ulimit_nofile: false,
        column_options: LedgerColumnOptions::default(),
    })?;

    let (bank_forks, _leader_schedule_cache, _starting_snapshot_hashes) = bank_forks_utils::load(
        &genesis_config,
        &blockstore,
        vec![],
        None,
        None,
        ProcessOptions::default(),
        None,
        None,
        None,
        None,
        Arc::new(AtomicBool::new(false))
    )?;

    let bank = bank_forks
        .read()
        .unwrap()
        .get(*snapshot_slot)
        .ok_or_else(|| anyhow!("Failed to get bank at slot {}", snapshot_slot))?
        .clone();

    Ok(bank)
}

async fn setup_test_environment() -> Result<(TempDir, Keypair, Pubkey, Pubkey)> {
    // Create temporary directory for test outputs
    let temp_dir = TempDir::new()?;
    println!("Created temp dir at: {:?}", temp_dir.path());

    // Create test keypair and program IDs
    let keypair = Keypair::new();
    let tip_distribution_program_id = Pubkey::new_unique();
    let tip_payment_program_id = Pubkey::new_unique();

    // Create ledger directory
    let ledger_dir = temp_dir.path().join("ledger");
    println!("Creating ledger directory at: {:?}", ledger_dir);
    fs::create_dir_all(&ledger_dir)?;

    Ok((temp_dir, keypair, tip_distribution_program_id, tip_payment_program_id))
}

#[tokio::test]
async fn test_complete_workflow() -> Result<()> {
    let (temp_dir, keypair, tip_distribution_program_id, tip_payment_program_id) =
        setup_test_environment().await?;

    // Print initial directory structure
    println!("Initial temp dir contents:");
    for entry in fs::read_dir(&temp_dir.path())? {
        println!("{:?}", entry?.path());
    }

    // 1. Setup test bank with validators
    let validator_keypairs: Vec<_> = (0..3).map(|_| ValidatorVoteKeypairs::new_rand()).collect();
    let GenesisConfigInfo { mut genesis_config, .. } = create_genesis_config_with_vote_accounts(
        1_000_000_000,
        &validator_keypairs.iter().collect::<Vec<_>>(),
        vec![1_000_000; 3]
    );

    // Create and prepare ledger directory
    let ledger_dir = temp_dir.path().join("ledger");
    fs::create_dir_all(&ledger_dir)?;

    // Create genesis files
    create_genesis_files(&ledger_dir, &genesis_config)?;

    // Create test bank
    let bank = Bank::new_for_tests(&genesis_config);
    let bank = Arc::new(bank);
    bank.freeze();

    // Create snapshot directory
    let snapshot_dir = temp_dir.path().join("snapshots");
    println!("Creating snapshot directory at: {:?}", snapshot_dir);
    fs::create_dir_all(&snapshot_dir)?;

    let slot = bank.slot();
    println!("Bank created at slot: {}", slot);

    // Initialize MerkleTreeGenerator
    println!("Initializing MerkleTreeGenerator...");
    let merkle_tree_generator = MerkleTreeGenerator::new(
        "http://localhost:8899",
        keypair,
        "test-ncn".to_string(),
        snapshot_dir.clone(),
        tip_distribution_program_id,
        Keypair::new(),
        Pubkey::find_program_address(&[b"config"], &tip_distribution_program_id).0
    )?;

    // Print ledger directory contents
    println!("\nLedger directory contents:");
    for entry in fs::read_dir(&ledger_dir)? {
        let entry = entry?;
        println!("{:?} - {} bytes", 
            entry.path(),
            entry.metadata()?.len()
        );
    }

    // Create snapshot directory structure
    let snapshot_path = snapshot_dir.join(format!("snapshot-{}", slot));
    println!("Creating snapshot directory at: {:?}", snapshot_path);
    fs::create_dir_all(&snapshot_path)?;

    // Write snapshot version file
    let snapshot_version_path = snapshot_dir.join("version");
    fs::write(&snapshot_version_path, "2.0.0")?;
    println!("Wrote snapshot version file: {:?}", snapshot_version_path);

    // Print snapshot directory contents
    println!("\nSnapshot directory contents:");
    for entry in fs::read_dir(&snapshot_dir)? {
        let entry = entry?;
        println!("{:?} - {} bytes", 
            entry.path(),
            entry.metadata()?.len()
        );
    }

    // Generate stake metadata
    let stake_meta_path = snapshot_dir.join(format!("stake-meta-{}.json", slot));
    println!("\nGenerating stake metadata at: {:?}", stake_meta_path);

    println!("Calling generate_stake_meta with:");
    println!("  ledger_dir: {:?}", ledger_dir);
    println!("  slot: {}", slot);
    println!("  tip_distribution_program_id: {}", tip_distribution_program_id);
    println!("  stake_meta_path: {:?}", stake_meta_path);
    println!("  tip_payment_program_id: {}", tip_payment_program_id);

    let _stake_meta = stake_meta_generator_workflow::generate_stake_meta(
        &ledger_dir,
        &slot,
        &tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        &tip_payment_program_id
    )?;

    // Generate and verify merkle trees
    println!("Generating merkle trees...");
    let file = std::fs::File::open(&stake_meta_path)?;
    let stake_meta_collection: StakeMetaCollection = serde_json::from_reader(file)?;

    let merkle_trees =
        merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta_collection).await?;

    assert_eq!(merkle_trees.generated_merkle_trees.len(), 3);
    println!("Generated {} merkle trees", merkle_trees.generated_merkle_trees.len());

    // Generate and verify meta merkle tree
    println!("Generating meta merkle tree...");
    let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;

    assert_eq!(meta_merkle_tree.validator_count(), 3);
    assert_eq!(meta_merkle_tree.epoch, bank.epoch());
    println!(
        "Meta merkle tree generated with {} validators at epoch {}",
        meta_merkle_tree.validator_count(),
        meta_merkle_tree.epoch
    );

    // Verify NCN upload (mock version)
    println!("Uploading to NCN...");
    merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;
    println!("Upload complete");

    Ok(())
}
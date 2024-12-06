use {
    solana_sdk::{
        signer::{keypair::Keypair, Signer},
        pubkey::Pubkey,
    },
    solana_runtime::{
        bank::Bank, 
        bank_forks::BankForks,
        genesis_utils::{create_genesis_config_with_vote_accounts, ValidatorVoteKeypairs},
    },
    tip_router_operator_cli::{
        merkle_tree::MerkleTreeGenerator,
        stake_meta_generator_workflow,
        StakeMetaCollection,
    },
    std::{path::PathBuf, fs},
    tempfile::TempDir,
    anyhow::Result,
};

async fn setup_test_environment() -> Result<(TempDir, Keypair, Pubkey, Pubkey)> {
    // Create temporary directory for test outputs
    let temp_dir = TempDir::new()?;
    
    // Create test keypair
    let keypair = Keypair::new();
    
    // Create program IDs
    let tip_distribution_program_id = Pubkey::new_unique();
    let tip_payment_program_id = Pubkey::new_unique();

    Ok((temp_dir, keypair, tip_distribution_program_id, tip_payment_program_id))
}

#[tokio::test]
async fn test_complete_workflow() -> Result<()> {
    let (temp_dir, keypair, tip_distribution_program_id, tip_payment_program_id) = 
        setup_test_environment().await?;

    // 1. Setup test bank with validators
    let validator_keypairs: Vec<_> = (0..3)
        .map(|_| ValidatorVoteKeypairs::new_rand())
        .collect();
    
    let validator_stakes = vec![1_000_000; 3];
    let genesis_config = create_genesis_config_with_vote_accounts(
        1_000_000_000,
        &validator_keypairs.iter().collect::<Vec<_>>(),
        validator_stakes,
    ).genesis_config;

    let (bank, _) = Bank::new_with_bank_forks_for_tests(&genesis_config);
    let slot = bank.slot();

    // 2. Create directories
    let snapshot_dir = temp_dir.path().join("snapshots");
    fs::create_dir_all(&snapshot_dir)?;
    let ledger_dir = temp_dir.path().join("ledger");
    fs::create_dir_all(&ledger_dir)?;

    // 3. Initialize MerkleTreeGenerator
    let merkle_tree_generator = MerkleTreeGenerator::new(
        "http://localhost:8899",
        keypair,
        "test-ncn".to_string(),
        snapshot_dir.clone(),
        tip_distribution_program_id,
        Keypair::new(),
        Pubkey::find_program_address(&[b"config"], &tip_distribution_program_id).0,
    )?;

    // 4. Generate stake metadata
    let stake_meta_path = snapshot_dir.join(format!("stake-meta-{}.json", slot));
    
    let stake_meta = stake_meta_generator_workflow::generate_stake_meta(
        &ledger_dir,
        &slot,
        &tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        &tip_payment_program_id,
    )?;

    // 5. Generate and verify merkle trees
    let file = std::fs::File::open(&stake_meta_path)?;
    let stake_meta_collection: StakeMetaCollection = serde_json::from_reader(file)?;
    
    let merkle_trees = merkle_tree_generator
        .generate_and_upload_merkle_trees(stake_meta_collection)
        .await?;

    assert_eq!(merkle_trees.generated_merkle_trees.len(), 3);

    // 6. Generate and verify meta merkle tree
    let meta_merkle_tree = merkle_tree_generator
        .generate_meta_merkle_tree(&merkle_trees)
        .await?;

    assert_eq!(meta_merkle_tree.validator_count(), 3);
    assert_eq!(meta_merkle_tree.epoch, bank.epoch());

    // 7. Verify NCN upload (mock version)
    merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

    Ok(())
}
use ::{
    anyhow::Result,
    solana_program_test::*,
    solana_sdk::{
        account::AccountSharedData,
        pubkey::Pubkey,
        rent::Rent,
        signature::{ Keypair, Signer },
        stake::{ self, state::{ Meta, Stake, StakeStateV2 }, stake_flags },
        vote::program::id as vote_program_id,
        genesis_config::GenesisConfig,
        signer::keypair::write_keypair_file,
    },
    std::{ fs, sync::Arc, time::Duration },
    tempfile::TempDir,
    tip_router_operator_cli::{
        claim_mev_workflow,
        merkle_root_generator_workflow,
        merkle_root_upload_workflow,
        stake_meta_generator_workflow,
        GeneratedMerkleTreeCollection,
    },
    solana_runtime::bank::Bank,
};

#[derive(Debug)]
struct ValidatorKeypairs {
    identity_keypair: Keypair,
    vote_keypair: Keypair,
    stake_keypair: Keypair,
}

async fn setup_validator_accounts(
    context: &mut ProgramTestContext,
    validator_keypairs: &[ValidatorKeypairs],
    tip_distribution_program_id: &Pubkey
) -> Result<()> {
    for keypairs in validator_keypairs {
        let vote_pubkey = keypairs.vote_keypair.pubkey();
        let stake_pubkey = keypairs.stake_keypair.pubkey();

        println!("Creating accounts for validator:");
        println!("Vote account: {}", vote_pubkey);
        println!("Stake account: {}", stake_pubkey);

        // Create vote account
        let vote_account = AccountSharedData::new(1_000_000, 200, &vote_program_id());
        context.set_account(&vote_pubkey, &vote_account);

        // Create stake account
        let stake_lamports = 1_000_000_000;
        let meta = Meta {
            rent_exempt_reserve: Rent::default().minimum_balance(
                std::mem::size_of::<StakeStateV2>()
            ),
            authorized: stake::state::Authorized::auto(&stake_pubkey),
            lockup: stake::state::Lockup::default(),
        };

        let stake = Stake {
            delegation: stake::state::Delegation {
                voter_pubkey: vote_pubkey,
                stake: stake_lamports - meta.rent_exempt_reserve,
                activation_epoch: 0,
                deactivation_epoch: u64::MAX,
                warmup_cooldown_rate: 0.25, // Using fixed value for test
            },
            credits_observed: 0,
        };

        let stake_state = StakeStateV2::Stake(meta, stake, stake_flags::StakeFlags::empty());
        let mut stake_account = AccountSharedData::new(
            stake_lamports,
            std::mem::size_of::<StakeStateV2>(),
            &stake::program::id()
        );
        stake_account.set_data(bincode::serialize(&stake_state)?);
        context.set_account(&stake_pubkey, &stake_account);

        // Create tip distribution account
        let seeds = &[b"tip_distribution", vote_pubkey.as_ref()];
        let (tip_distribution_address, _) = Pubkey::find_program_address(
            seeds,
            tip_distribution_program_id
        );

        let tip_distribution_account = AccountSharedData::new(
            1_000_000,
            0,
            tip_distribution_program_id
        );
        context.set_account(&tip_distribution_address, &tip_distribution_account);
    }

    Ok(())
}

#[tokio::test]
async fn test_full_workflow() -> Result<()> {
    env_logger::init();

    // Create temporary directory for test files
    let temp_dir = TempDir::new()?;
    println!("Created temp dir at {:?}", temp_dir.path());

    // Setup test validator
    let mut program_test = ProgramTest::default();
    let tip_distribution_program_id = Pubkey::new_unique();
    let tip_payment_program_id = Pubkey::new_unique();

    // Create validator keypairs
    let num_validators = 3;
    let mut validator_keypairs = Vec::with_capacity(num_validators);
    for _ in 0..num_validators {
        validator_keypairs.push(ValidatorKeypairs {
            identity_keypair: Keypair::new(),
            vote_keypair: Keypair::new(),
            stake_keypair: Keypair::new(),
        });
    }

    // Start test validator
    let mut context = program_test.start_with_context().await;
    let context_keypair = Keypair::new();

    // Setup validator accounts
    setup_validator_accounts(
        &mut context,
        &validator_keypairs,
        &tip_distribution_program_id
    ).await?;

    // Create snapshot directory and output files
    let snapshot_dir = temp_dir.path().join("snapshots");
    fs::create_dir_all(&snapshot_dir)?;

    let stake_meta_path = snapshot_dir.join("stake-meta.json");
    let merkle_tree_path = snapshot_dir.join("merkle-tree.json");

    // Save keypair to file for merkle root upload
    println!("Saving keypair to file...");
    let keypair_path = temp_dir.path().join("test-keypair.json");
    write_keypair_file(&context_keypair, &keypair_path).expect("Failed to write keypair file");

    println!("\nTesting workflow components:");

    // 1. Generate stake metadata
    println!("1. Generating stake metadata...");

    // Create a Bank instance
    let genesis_config = GenesisConfig::default();
    let bank = Bank::new_for_tests(&genesis_config);

    // Add the accounts to the bank
    for keypairs in &validator_keypairs {
        let vote_pubkey = keypairs.vote_keypair.pubkey();
        let stake_pubkey = keypairs.stake_keypair.pubkey();

        // Create vote account
        let vote_account = AccountSharedData::new(1_000_000, 200, &vote_program_id());
        bank.store_account(&vote_pubkey, &vote_account);

        // Create stake account
        let stake_lamports = 1_000_000_000;
        let meta = Meta {
            rent_exempt_reserve: Rent::default().minimum_balance(
                std::mem::size_of::<StakeStateV2>()
            ),
            authorized: stake::state::Authorized::auto(&stake_pubkey),
            lockup: stake::state::Lockup::default(),
        };

        let stake = Stake {
            delegation: stake::state::Delegation {
                voter_pubkey: vote_pubkey,
                stake: stake_lamports - meta.rent_exempt_reserve,
                activation_epoch: 0,
                deactivation_epoch: u64::MAX,
                warmup_cooldown_rate: stake::state::warmup_cooldown_rate(0, None),
            },
            credits_observed: 0,
        };

        let stake_state = StakeStateV2::Stake(meta, stake, stake_flags::StakeFlags::empty());
        let mut stake_account = AccountSharedData::new(
            stake_lamports,
            std::mem::size_of::<StakeStateV2>(),
            &stake::program::id()
        );
        stake_account.set_data(bincode::serialize(&stake_state).unwrap());
        bank.store_account(&stake_pubkey, &stake_account);
    }

    let stake_meta = stake_meta_generator_workflow::generate_stake_meta_from_bank(
        &Arc::new(bank),
        &tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
    )?;

    assert_eq!(stake_meta.stake_metas.len(), num_validators);

    // 2. Generate merkle trees
    println!("2. Generating merkle trees...");
    merkle_root_generator_workflow::generate_merkle_root(
        &stake_meta_path,
        &merkle_tree_path,
        "http://localhost:8899"
    )?;

    // Before uploading merkle roots
    println!("Merkle tree file exists: {}", merkle_tree_path.exists());
    println!("Keypair file exists: {}", keypair_path.exists());

    // 3. Upload merkle roots
    println!("3. Uploading merkle roots...");
    println!("Using keypair path: {:?}", keypair_path);
    match merkle_root_upload_workflow::upload_merkle_root(
        &merkle_tree_path,
        &keypair_path,
        "http://localhost:8899",
        &tip_distribution_program_id,
        100,
        64
    ).await {
        Ok(_) => (),  // Return unit type
        Err(e) => {
            eprintln!("Error uploading merkle root: {}", e);
            panic!("Failed to upload merkle root");  // Or handle error appropriately
        }
    };

    // 4. Test claiming tips
    println!("4. Testing tip claiming...");
    let merkle_trees: GeneratedMerkleTreeCollection = serde_json::from_reader(
        fs::File::open(&merkle_tree_path)?
    )?;

    let claim_result = claim_mev_workflow::claim_mev_tips(
        &merkle_trees,
        "http://localhost:8899".to_string(),
        tip_distribution_program_id,
        Arc::new(context_keypair),
        Duration::from_secs(10),
        1
    ).await;

    assert!(claim_result.is_ok(), "Tip claiming failed: {:?}", claim_result.err());

    Ok(())
}

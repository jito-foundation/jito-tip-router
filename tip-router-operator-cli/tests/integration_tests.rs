use {
    solana_sdk::{
        signer::keypair::Keypair,
        pubkey::Pubkey,
        slot_history::Slot,
        signer::Signer,
        account::ReadableAccount,
        account::AccountSharedData,
        stake::state::{
            Delegation,
            StakeState,
            Meta,
            Authorized,
            Lockup,
            Stake,  // Add this import
        },

    },
    solana_runtime::{
        bank::Bank,
        genesis_utils::{
            create_genesis_config_with_vote_accounts,
            ValidatorVoteKeypairs,
            GenesisConfigInfo,
        },
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
    solana_accounts_db::accounts_index::ScanConfig,
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
    let mut file = std::fs::OpenOptions::new().write(true).create_new(true).open(&genesis_path)?;

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
    // Create temporary directory for test outputs
    let (temp_dir, keypair, tip_distribution_program_id, tip_payment_program_id) =
        setup_test_environment().await?;

    // Setup test bank with validators
    let validator_keypairs: Vec<_> = (0..3).map(|_| ValidatorVoteKeypairs::new_rand()).collect();
    let GenesisConfigInfo { mut genesis_config, .. } = create_genesis_config_with_vote_accounts(
        1_000_000_000,
        &validator_keypairs.iter().collect::<Vec<_>>(),
        vec![10_000_000; 3]
    );

    // Create and prepare ledger directory
    let ledger_dir = temp_dir.path().join("ledger");
    fs::create_dir_all(&ledger_dir)?;
    create_genesis_files(&ledger_dir, &genesis_config)?;

    // Create test bank and process some slots
    let mut bank = Arc::new(Bank::new_for_tests(&genesis_config));
    
    println!("\nSetting up validator stakes:");
    // Activate stakes and create delegations for all validators
    for keypairs in &validator_keypairs {
        let vote_pubkey = keypairs.vote_keypair.pubkey();
        let stake_pubkey = keypairs.stake_keypair.pubkey();
        
        // Create stake account
        let stake_lamports = 10_000_000;
        let mut stake_account = AccountSharedData::new(
            stake_lamports,
            std::mem::size_of::<StakeState>(),
            &solana_sdk::stake::program::id(),
        );

        println!("Creating stake account {} for vote account {}", stake_pubkey, vote_pubkey);
        println!("Stake program ID: {}", solana_sdk::stake::program::id());

        // Create delegation
        let delegation = Delegation {
            voter_pubkey: vote_pubkey,
            stake: stake_lamports,
            activation_epoch: 0,
            deactivation_epoch: std::u64::MAX,
            warmup_cooldown_rate: 0.25,
        };

        let meta = Meta {
            rent_exempt_reserve: 0,
            authorized: Authorized {
                staker: stake_pubkey,
                withdrawer: stake_pubkey,
            },
            lockup: Lockup::default(),
        };

        let stake = Stake {
            delegation,
            credits_observed: 0,
        };

        let stake_state = StakeState::Stake(meta, stake);
        
        // Serialize stake state into account data
        let data = bincode::serialize(&stake_state)?;
        stake_account.set_data(data);
        
        // Store stake account
        bank.store_account(&stake_pubkey, &stake_account);
        
        // Verify stake account was stored
        if let Some(stored_account) = bank.get_account(&stake_pubkey) {
            println!("Verified stake account {} exists with {} lamports", 
                stake_pubkey, 
                stored_account.lamports()
            );
            
            // Try to deserialize and verify stake state
            if let Ok(StakeState::Stake(meta, stake)) = bincode::deserialize(stored_account.data()) {
                println!("  Delegation: {} stake to {}", 
                    stake.delegation.stake,
                    stake.delegation.voter_pubkey
                );
            } else {
                println!("  Warning: Could not deserialize stake state!");
            }
        } else {
            println!("  Warning: Stake account not found after storing!");
        }
        
        // Create and store tip distribution account
        let seeds = &[b"tip_distribution", vote_pubkey.as_ref()];
        let (tip_distribution_address, _) = 
            Pubkey::find_program_address(seeds, &tip_distribution_program_id);
            
        let tip_distribution_account = AccountSharedData::new(
            1_000_000,
            0,
            &tip_distribution_program_id,
        );
        
        println!("Creating tip distribution account at {}", tip_distribution_address);
        bank.store_account(&tip_distribution_address, &tip_distribution_account);
    }

     // Process slots to activate stakes
     println!("\nProcessing slots to activate stakes:");
     let slots_per_epoch = genesis_config.epoch_schedule.slots_per_epoch;
     let warmup_epochs = 2; // Need at least 2 epochs for stakes to fully activate
     
     for slot in 1..=slots_per_epoch * warmup_epochs {
         let child = Bank::new_from_parent(bank.clone(), &Pubkey::default(), slot);
         bank = Arc::new(child);
         
         // Only log at epoch boundaries
         if slot % slots_per_epoch == 0 {
             println!("Epoch advanced to {}", bank.epoch());
             
             // Print stake activation status
             for keypairs in &validator_keypairs {
                 let stake_pubkey = keypairs.stake_keypair.pubkey();
                 if let Some(stake_account) = bank.get_account(&stake_pubkey) {
                     if let Ok(StakeState::Stake(_, stake)) = bincode::deserialize(stake_account.data()) {
                         println!("Stake {} activation status: {}% active", 
                             stake_pubkey,
                             (stake.delegation.stake as f64 / stake_account.lamports() as f64) * 100.0
                         );
                     }
                 }
             }
         }
     }
     
    println!("\nBank state after processing:");
    println!("Slot: {}", bank.slot());
    println!("Epoch: {}", bank.epoch());
    println!("Active stakes count: {}", bank.stakes_cache.stakes().stake_delegations().len());
    
    // Print all stake accounts in bank
    let stake_program_id = solana_sdk::stake::program::id();
    let scan_config = ScanConfig::default();
    let stake_accounts = bank.get_program_accounts(&stake_program_id, &scan_config)?;
    
    for (pubkey, account) in stake_accounts {
        println!("Found stake account: {}", pubkey);
        if let Ok(StakeState::Stake(meta, stake)) = bincode::deserialize(account.data()) {
            println!("  Delegation: {} stake to {}", 
                stake.delegation.stake,
                stake.delegation.voter_pubkey
            );
        }
    }

    let slot = bank.slot();
    
    // Create snapshot directory
    let snapshot_dir = temp_dir.path().join("snapshots");
    fs::create_dir_all(&snapshot_dir)?;
    
    // Initialize MerkleTreeGenerator
    println!("\nInitializing MerkleTreeGenerator with:");
    println!("  RPC URL: http://localhost:8899");
    println!("  Snapshot dir: {:?}", snapshot_dir);
    println!("  Tip distribution program ID: {}", tip_distribution_program_id);

    let merkle_tree_generator = MerkleTreeGenerator::new(
        "http://localhost:8899",
        keypair,
        "test-ncn".to_string(),
        snapshot_dir.clone(),
        tip_distribution_program_id,
        Keypair::new(),
        Pubkey::find_program_address(&[b"config"], &tip_distribution_program_id).0,
    )?;

    // Generate stake metadata
    let stake_meta_path = snapshot_dir.join(format!("stake-meta-{}.json", slot));
    println!("\nGenerating stake meta at path: {:?}", stake_meta_path);
    
    let stake_meta = stake_meta_generator_workflow::generate_stake_meta_from_bank(
        &bank,
        &tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        &tip_payment_program_id,
    )?;

    // Print debug info
    println!("\nGenerated stake meta with {} validators", stake_meta.stake_metas.len());
    for (i, meta) in stake_meta.stake_metas.iter().enumerate() {
        println!("Validator {}: Vote account: {}, Stake: {}, Node: {}", 
            i, 
            meta.validator_vote_account, 
            meta.total_delegated,
            meta.validator_node_pubkey
        );
        
        if meta.delegations.is_empty() {
            println!("  Warning: No delegations for this validator");
        }
        
        if meta.maybe_tip_distribution_meta.is_none() {
            println!("  Warning: No tip distribution meta for this validator");
        }
    }

    // Generate merkle trees
    println!("\nGenerating merkle trees...");
    let merkle_trees = merkle_tree_generator
        .generate_and_upload_merkle_trees(stake_meta)
        .await?;

    println!("Generated {} merkle trees", merkle_trees.generated_merkle_trees.len());
    
    // Print merkle tree details
    for (i, tree) in merkle_trees.generated_merkle_trees.iter().enumerate() {
        println!("Merkle tree {}: {} nodes, max claim: {}", 
            i, 
            tree.tree_nodes.len(),
            tree.max_total_claim
        );
    }

    assert_eq!(merkle_trees.generated_merkle_trees.len(), 3);

    Ok(())
}
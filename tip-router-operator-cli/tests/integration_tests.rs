use ::{
    anyhow::Result,
    log::info,
    solana_program_test::*,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{ Keypair, Signer },
        stake::{ self, state::{ Meta, Stake, StakeStateV2 } },
        stake::stake_flags::StakeFlags, // Keep only one StakeFlags import
        vote::program::id as vote_program_id,
        account::AccountSharedData,
        sysvar::rent::Rent,
    },
    ellipsis_client::EllipsisClient,
    std::{ path::PathBuf, sync::Arc, time::Duration, fs },
    solana_client::nonblocking::rpc_client::RpcClient, // Add this
    tip_router_operator_cli::{
        merkle_tree::MerkleTreeGenerator,
        claim_mev_workflow,
        merkle_root_generator_workflow,
        stake_meta_generator_workflow,
        snapshot::SnapshotCreator,
        StakeMetaCollection,
        GeneratedMerkleTreeCollection,
        TipDistributionMeta,
        Delegation,
        StakeMeta,
    },
};

pub struct ValidatorKeypairs {
    pub vote_keypair: Keypair,
    pub stake_keypair: Keypair,
}

impl ValidatorKeypairs {
    pub fn new() -> Self {
        Self {
            vote_keypair: Keypair::new(),
            stake_keypair: Keypair::new(),
        }
    }
}

struct TestContext {
    rpc_client: Arc<EllipsisClient>,
    keypair: Keypair,
    keypair_path: PathBuf,
    snapshot_dir: PathBuf,
    ledger_dir: PathBuf,
    tip_distribution_program_id: Pubkey,
    tip_payment_program_id: Pubkey,
    ncn_address: Pubkey,
    temp_dir: tempfile::TempDir,
    program_context: ProgramTestContext,
    validator_keypairs: Vec<ValidatorKeypairs>,
    // Add these new fields
    vote_pubkey: Pubkey,
    stake_pubkey: Pubkey,
    tip_distribution_address: Pubkey,
}

impl TestContext {
    async fn new() -> Result<Self> {
        // Create program test
        let mut program_test = ProgramTest::default();
        let program_context = program_test.start_with_context().await;

        // Create temporary directories
        let temp_dir = tempfile::tempdir()?;
        let snapshot_dir = temp_dir.path().join("snapshots");
        let ledger_dir = temp_dir.path().join("ledger");
        fs::create_dir_all(&snapshot_dir)?;
        fs::create_dir_all(&ledger_dir)?;

        // Generate test keypair
        let keypair = Keypair::new();
        let keypair_path = temp_dir.path().join("keypair.json");
        fs::write(&keypair_path, keypair.to_bytes())?;

        // Create test validator keypairs
        let validator_keypairs = vec![ValidatorKeypairs::new(), ValidatorKeypairs::new()];

        let tip_distribution_program_id = Pubkey::new_unique();

        // Setup RPC client
        // Setup RPC client
        let rpc = solana_client::nonblocking::rpc_client::RpcClient::new(
            "http://localhost:8899".to_string()
        );
        let rpc_client = Arc::new(
            EllipsisClient::from_rpc(
                solana_client::rpc_client::RpcClient::new("http://localhost:8899".to_string()),
                &keypair
            )?
        );

        // Generate the new pubkeys
        let vote_pubkey = Pubkey::new_unique();
        let stake_pubkey = Pubkey::new_unique();
        let tip_distribution_address = Pubkey::new_unique();

        Ok(Self {
            rpc_client,
            keypair,
            keypair_path,
            snapshot_dir,
            ledger_dir,
            tip_distribution_program_id,
            tip_payment_program_id: Pubkey::new_unique(),
            ncn_address: Pubkey::new_unique(),
            temp_dir,
            program_context,
            validator_keypairs,
            // Add the new fields
            vote_pubkey,
            stake_pubkey,
            tip_distribution_address,
        })
    }

    fn create_test_stake_meta(&self) -> Result<StakeMetaCollection> {
        // Create a sample stake meta for testing
        let stake_meta = StakeMeta {
            validator_vote_account: self.vote_pubkey,
            validator_node_pubkey: self.stake_pubkey,
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                total_tips: 1_000_000,
                merkle_root_upload_authority: Pubkey::new_unique(),
                tip_distribution_pubkey: self.tip_distribution_program_id,
                validator_fee_bps: 1000, // 10% in basis points
            }),
            delegations: vec![Delegation {
                stake_account_pubkey: self.stake_pubkey,
                staker_pubkey: self.vote_pubkey,
                withdrawer_pubkey: self.vote_pubkey,
                lamports_delegated: 1_000_000,
            }],
            total_delegated: 1_000_000,
            commission: 10,
        };

        // Create a collection with our stake meta
        let stake_meta_collection = StakeMetaCollection {
            epoch: 0,
            stake_metas: vec![stake_meta],
            bank_hash: "test_bank_hash".to_string(),
            slot: 0,
            tip_distribution_program_id: self.tip_distribution_program_id, // Add this field
        };

        Ok(stake_meta_collection)
    }

    async fn wait_for_next_epoch(&self) -> Result<()> {
        let current_epoch = self.rpc_client.get_epoch_info()?.epoch;

        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            let new_epoch = self.rpc_client.get_epoch_info()?.epoch;

            if new_epoch > current_epoch {
                info!("New epoch detected: {} -> {}", current_epoch, new_epoch);
                return Ok(());
            }
        }
    }

    async fn get_previous_epoch_last_slot(&self) -> Result<u64> {
        let epoch_info = self.rpc_client.get_epoch_info()?;
        let current_slot = epoch_info.absolute_slot;
        let slot_index = epoch_info.slot_index;

        let epoch_start_slot = current_slot - slot_index;
        let previous_epoch_final_slot = epoch_start_slot - 1;

        Ok(previous_epoch_final_slot)
    }
}

async fn setup_validator_accounts(
    context: &mut ProgramTestContext,
    validator_keypairs: &[ValidatorKeypairs],
    tip_distribution_program_id: &Pubkey
) -> Result<()> {
    for keypairs in validator_keypairs {
        let vote_pubkey = keypairs.vote_keypair.pubkey();
        let stake_pubkey = keypairs.stake_keypair.pubkey();

        info!("Creating accounts for validator:");
        info!("Vote account: {}", vote_pubkey);
        info!("Stake account: {}", stake_pubkey);

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
                warmup_cooldown_rate: 0.25,
            },
            credits_observed: 0,
        };

        let stake_state = StakeStateV2::Stake(meta, stake, StakeFlags::empty());
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

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_processing() -> Result<()> {
    let context = TestContext::new().await?;

    // 1. Create snapshot
    info!("1. Testing snapshot creation...");

    let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;
    let rpc_url = context.rpc_client.url().to_string();
    // Define merkle_tree_path here since we'll need it later
    let merkle_tree_path = context.snapshot_dir.join("merkle-trees");

    let snapshot_creator = SnapshotCreator::new(
        &rpc_url,
        context.snapshot_dir.to_str().unwrap().to_string(),
        5,
        "bzip2".to_string(),
        keypair_copy,
        context.ledger_dir.clone()
    )?;

    let slot = context.get_previous_epoch_last_slot().await?;
    snapshot_creator.create_snapshot(slot).await?;
    // 2. Generate stake metadata
    info!("2. Testing stake metadata generation...");
    let stake_meta_path = context.snapshot_dir.join("stake-meta.json");

    stake_meta_generator_workflow::generate_stake_meta(
        &context.ledger_dir,
        &slot,
        &context.tip_distribution_program_id,
        stake_meta_path.to_str().unwrap(),
        &context.tip_payment_program_id
    )?;

    let stake_meta = context.create_test_stake_meta()?;

    // 3. Create merkle trees
    info!("3. Testing merkle tree generation...");
    let merkle_tree_path = context.snapshot_dir.join("merkle-trees");
    merkle_root_generator_workflow::generate_merkle_root(
        &stake_meta_path,
        &merkle_tree_path,
        &rpc_url
    )?;

    let keypair_copy2 = Keypair::from_bytes(&context.keypair.to_bytes())?;
    let snapshot_creator = SnapshotCreator::new(
        &rpc_url,
        context.snapshot_dir.to_str().unwrap().to_string(),
        5,
        "bzip2".to_string(),
        keypair_copy2,
        context.ledger_dir.clone()
    )?;

    let keypair_copy3 = Keypair::from_bytes(&context.keypair.to_bytes())?;
    // 4. Initialize MerkleTreeGenerator
    info!("4. Testing MerkleTreeGenerator initialization...");
    let merkle_tree_generator = MerkleTreeGenerator::new(
        &rpc_url,
        keypair_copy3,
        context.ncn_address,
        merkle_tree_path.clone(),
        context.tip_distribution_program_id,
        Keypair::new(),
        Pubkey::new_unique()
    )?;

    // 5. Generate and upload merkle trees
    info!("5. Testing merkle tree generation and upload...");
    let merkle_trees = merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta).await?;

    // 6. Generate meta merkle tree
    info!("6. Testing meta merkle tree generation...");
    let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;

    // 7. Upload to NCN
    info!("7. Testing NCN upload...");
    merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

    // 8. Test claiming
    info!("8. Testing claiming capability...");
    claim_mev_workflow::claim_mev_tips(
        &merkle_trees,
        rpc_url,
        context.tip_distribution_program_id,
        Arc::new(context.keypair),
        Duration::from_secs(10),
        1
    ).await?;

    Ok(())
}

// Additional test cases remain similar but use EllipsisClient instead of RpcClient
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_merkle_tree_generation() -> Result<()> {
    let context = TestContext::new().await?;
    let stake_meta = context.create_test_stake_meta()?;
    let rpc_url = context.rpc_client.url().to_string();

    let merkle_tree_path = context.snapshot_dir.join("merkle-trees");
    merkle_root_generator_workflow::generate_merkle_root(
        &context.snapshot_dir.join("stake-meta.json"),
        &merkle_tree_path,
        &rpc_url
    )?;

    assert!(merkle_tree_path.exists());

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_meta_merkle_tree() -> Result<()> {
    let context = TestContext::new().await?;

    let rpc_url = context.rpc_client.url().to_string();
    let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;

    let merkle_tree_generator = MerkleTreeGenerator::new(
        &rpc_url,
        keypair_copy,
        context.ncn_address,
        context.snapshot_dir.clone(),
        context.tip_distribution_program_id,
        Keypair::new(),
        Pubkey::new_unique()
    )?;

    let merkle_trees = GeneratedMerkleTreeCollection {
        epoch: 0,
        generated_merkle_trees: vec![],
        bank_hash: "0".to_string(),
        slot: 0, // Add slot
    };

    let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;

    assert!(meta_merkle_tree.root != [0; 32]);

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_ncn_upload() -> Result<()> {
    let context = TestContext::new().await?;
    let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;
    let rpc_url = context.rpc_client.url().to_string();

    let merkle_tree_generator = MerkleTreeGenerator::new(
        &rpc_url,
        keypair_copy,
        context.ncn_address,
        context.snapshot_dir.clone(),
        context.tip_distribution_program_id,
        Keypair::new(),
        Pubkey::new_unique()
    )?;

    let merkle_trees = GeneratedMerkleTreeCollection {
        epoch: 0,
        generated_merkle_trees: vec![],
        bank_hash: "0".to_string(),
        slot: 0,
    };

    let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;

    merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_claim_mev_tips() -> Result<()> {
    let context = TestContext::new().await?;
    let stake_meta = context.create_test_stake_meta()?;

    let rpc_url = context.rpc_client.url().to_string();
    let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;

    let merkle_tree_generator = MerkleTreeGenerator::new(
        &rpc_url,
        keypair_copy,
        context.ncn_address,
        context.snapshot_dir.clone(),
        context.tip_distribution_program_id,
        Keypair::new(),
        Pubkey::new_unique()
    )?;

    let merkle_trees = merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta).await?;

    claim_mev_workflow::claim_mev_tips(
        &merkle_trees,
        rpc_url,
        context.tip_distribution_program_id,
        Arc::new(context.keypair),
        Duration::from_secs(10),
        1
    ).await?;

    Ok(())
}

async fn advance_test_epoch(context: &mut ProgramTestContext, slots: u64) -> Result<()> {
    for _ in 0..slots {
        let root_slot = context.banks_client.get_root_slot().await?;
        context.warp_to_slot(root_slot + 1)?;
    }
    Ok(())
}

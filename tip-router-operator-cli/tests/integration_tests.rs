use ::{
    anyhow::Result,
    log::info,
    solana_program_test::*,
    solana_sdk::{
        pubkey::Pubkey,
        signature::{ Keypair, Signer },
        stake::{ self, state::{ Meta, Stake, StakeStateV2 } },
        stake::stake_flags::StakeFlags,
        account::AccountSharedData,
        sysvar::rent::Rent,
        hash::Hash,
        account::WritableAccount,
        vote::{
            state::VoteState,
            authorized_voters::AuthorizedVoters,
            program::id as vote_program_id,
        },
        vote::state::VoteInit,
        sysvar::clock::Clock,
        account::Account as SolanaAccount,
        genesis_config::GenesisConfig,
        signer::keypair::read_keypair_file,
    },
    solana_client::rpc_response::StakeActivationState,
    ellipsis_client::EllipsisClient,
    std::error::Error as StdError,
    std::io::{ BufRead, Error as IoError },
    std::{ path::PathBuf, sync::Arc, time::Duration, fs },
    tip_router_operator_cli::{
        merkle_tree::MerkleTreeGenerator,
        claim_mev_workflow,
        merkle_root_generator_workflow,
        stake_meta_generator_workflow,
        snapshot::SnapshotCreator,
        StakeMetaCollection,
        GeneratedMerkleTree,
        GeneratedMerkleTreeCollection,
        TipDistributionMeta,
        Delegation,
        StakeMeta,
        TreeNode,
    },
    solana_client::rpc_client::RpcClient,
    serde::Serialize,
    solana_sdk::vote::state::VoteStateVersions,
};

// Update ValidatorKeypairs struct to include identity keypair
pub struct ValidatorKeypairs {
    pub vote_keypair: Keypair,
    pub stake_keypair: Keypair,
    pub identity_keypair: Keypair,
}

impl ValidatorKeypairs {
    pub fn new() -> Self {
        Self {
            vote_keypair: Keypair::new(),
            stake_keypair: Keypair::new(),
            identity_keypair: Keypair::new(),
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
    validator_keypairs: Vec<ValidatorKeypairs>,
    // Add these new fields
    vote_pubkey: Pubkey,
    stake_pubkey: Pubkey,
    tip_distribution_address: Pubkey,
}

impl TestContext {
    async fn new() -> Result<Self> {
        // Read validator info from file
        let validator_file = std::fs::File
            ::open("scripts/validators.txt")
            .map_err(|e| anyhow::anyhow!("Failed to open validators.txt: {}", e))?;
        let reader = std::io::BufReader::new(validator_file);

        let mut validator_keypairs = Vec::new();

        // Each line contains a vote account pubkey
        for (i, line) in reader.lines().enumerate() {
            let vote_pubkey_str = line
                .map_err(|e| anyhow::anyhow!("Failed to read line: {}", e))?
                .trim()
                .to_string();

            // Load the corresponding keypairs from the test-validator-keys directory
            let vote_keypair = read_keypair_file(
                format!("scripts/test-validator-keys/vote_{}.json", i + 1)
            ).map_err(|e| anyhow::anyhow!("Failed to read vote keypair: {}", e))?;

            let stake_keypair = read_keypair_file(
                format!("scripts/test-validator-keys/stake_{}.json", i + 1)
            ).map_err(|e| anyhow::anyhow!("Failed to read stake keypair: {}", e))?;

            let identity_keypair = read_keypair_file(
                format!("scripts/test-validator-keys/identity_{}.json", i + 1)
            ).map_err(|e| anyhow::anyhow!("Failed to read identity keypair: {}", e))?;

            validator_keypairs.push(ValidatorKeypairs {
                vote_keypair,
                stake_keypair,
                identity_keypair,
            });
        }

        let tip_distribution_program_id = Pubkey::new_unique();

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

        // Setup RPC client
        let rpc_client = Arc::new(
            EllipsisClient::from_rpc(RpcClient::new("http://localhost:8899".to_string()), &keypair)?
        );

        // Use the first validator's keypairs for the test context
        let vote_pubkey = validator_keypairs[0].vote_keypair.pubkey();
        let stake_pubkey = validator_keypairs[0].stake_keypair.pubkey();
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
            validator_keypairs,
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

        // Handle case where we're in the first epoch
        if current_slot < slot_index {
            return Ok(0);
        }

        let epoch_start_slot = current_slot - slot_index;
        let previous_epoch_final_slot = epoch_start_slot.saturating_sub(1);

        Ok(previous_epoch_final_slot)
    }

    async fn wait_for_stakes_to_activate(&self) -> Result<()> {
        info!("Waiting for stakes to activate...");
        for validator in &self.validator_keypairs {
            loop {
                let stake_status = self.rpc_client
                    .get_stake_activation(validator.stake_keypair.pubkey(), None)?;
                
                if stake_status.state == StakeActivationState::Active {
                    break;
                }
                
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
        info!("All stakes are now active");
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_epoch_processing() -> Result<()> {
    let context = TestContext::new().await?;
    
    // Wait for stakes to activate before proceeding
    context.wait_for_stakes_to_activate().await?;
    
    // Rest of the test...
    info!("1. Testing snapshot creation...");

    let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;
    let rpc_url = context.rpc_client.url().to_string();
    // Define merkle_tree_path here since we'll need it later
    let merkle_tree_path = context.snapshot_dir.join("merkle-trees");

    let local_ledger_dir = PathBuf::from("test-ledger");

    let snapshot_creator = SnapshotCreator::new(
        &rpc_url,
        context.snapshot_dir.to_str().unwrap().to_string(),
        5,
        "bzip2".to_string(),
        keypair_copy,
        local_ledger_dir.clone()  // Add .clone() here
    )?;

    let slot = context.get_previous_epoch_last_slot().await?;
    snapshot_creator.create_snapshot(slot).await?;
    // 2. Generate stake metadata
    info!("2. Testing stake metadata generation...");
    let stake_meta_path = context.snapshot_dir.join("stake-meta.json");

    stake_meta_generator_workflow::generate_stake_meta(
        &local_ledger_dir,
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
    // 4. Initialize MerkleTreeGenerator
    info!("4. Testing MerkleTreeGenerator initialization...");
    let merkle_tree_generator = MerkleTreeGenerator::new(
        &rpc_url,
        keypair_copy2,
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
// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_merkle_tree_generation() -> Result<()> {
//     let context = TestContext::new().await?;
//     let stake_meta = context.create_test_stake_meta()?;

//     // Rest of the test remains the same...
//     std::fs::create_dir_all(&context.snapshot_dir)?;
//     let stake_meta_path = context.snapshot_dir.join("stake-meta.json");

//     // Write stake meta to file
//     std::fs::write(&stake_meta_path, serde_json::to_string(&stake_meta)?)?;

//     let rpc_url = context.rpc_client.url().to_string();
//     let merkle_tree_path = context.snapshot_dir.join("merkle-trees");

//     // Create merkle tree directory
//     std::fs::create_dir_all(&merkle_tree_path)?;

//     merkle_root_generator_workflow::generate_merkle_root(
//         &stake_meta_path,
//         &merkle_tree_path,
//         &rpc_url
//     )?;

//     assert!(merkle_tree_path.exists());

//     Ok(())
// }

// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_ncn_upload() -> Result<()> {
//     let context = TestContext::new().await?;
//     let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;
//     let rpc_url = context.rpc_client.url().to_string();

//     // Create necessary directories
//     std::fs::create_dir_all(&context.snapshot_dir)?;

//     let merkle_tree_generator = MerkleTreeGenerator::new(
//         &rpc_url,
//         keypair_copy,
//         context.ncn_address,
//         context.snapshot_dir.clone(),
//         context.tip_distribution_program_id,
//         Keypair::new(),
//         Pubkey::new_unique()
//     )?;

//     // Create test merkle trees with actual data
//     let merkle_trees = GeneratedMerkleTreeCollection {
//         epoch: 0,
//         generated_merkle_trees: vec![GeneratedMerkleTree {
//             tip_distribution_account: Pubkey::new_unique(),
//             merkle_root_upload_authority: Pubkey::new_unique(),
//             merkle_root: Hash::new_unique(),
//             tree_nodes: vec![TreeNode {
//                 proof: Some(vec![[0u8; 32]; 32]), // Changed to match expected type
//                 claimant: Pubkey::new_unique(),
//                 claim_status_pubkey: Pubkey::new_unique(),
//                 claim_status_bump: 255,
//                 staker_pubkey: Pubkey::new_unique(),
//                 withdrawer_pubkey: Pubkey::new_unique(),
//                 amount: 1000,
//             }],
//             max_total_claim: 1000,
//             max_num_nodes: 1,
//         }],
//         bank_hash: "test_bank_hash".to_string(),
//         slot: 0,
//     };

//     let meta_merkle_tree = merkle_tree_generator.generate_meta_merkle_tree(&merkle_trees).await?;
//     merkle_tree_generator.upload_to_ncn(&meta_merkle_tree).await?;

//     Ok(())
// }

// #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
// async fn test_claim_mev_tips() -> Result<()> {
//     let context = TestContext::new().await?;
//     let stake_meta = context.create_test_stake_meta()?;

//     let rpc_url = context.rpc_client.url().to_string();
//     let keypair_copy = Keypair::from_bytes(&context.keypair.to_bytes())?;

//     let merkle_tree_generator = MerkleTreeGenerator::new(
//         &rpc_url,
//         keypair_copy,
//         context.ncn_address,
//         context.snapshot_dir.clone(),
//         context.tip_distribution_program_id,
//         Keypair::new(),
//         Pubkey::new_unique()
//     )?;

//     let merkle_trees = merkle_tree_generator.generate_and_upload_merkle_trees(stake_meta).await?;

//     claim_mev_workflow::claim_mev_tips(
//         &merkle_trees,
//         rpc_url,
//         context.tip_distribution_program_id,
//         Arc::new(context.keypair),
//         Duration::from_secs(10),
//         1
//     ).await?;

//     Ok(())
// }

// async fn advance_test_epoch(context: &mut ProgramTestContext, slots: u64) -> Result<()> {
//     for _ in 0..slots {
//         let root_slot = context.banks_client.get_root_slot().await?;
//         context.warp_to_slot(root_slot + 1)?;
//     }
//     Ok(())
// }

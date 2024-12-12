use {
    meta_merkle_tree::{
        meta_merkle_tree::MetaMerkleTree,
        generated_merkle_tree::GeneratedMerkleTreeCollection as MetaMerkleTreeCollection,
    },
    solana_program_test::*,
    solana_program::stake::state::StakeStateV2,
    solana_sdk::{
        signature::{ Keypair, Signer },
        pubkey::Pubkey,
        system_instruction,
        transaction::Transaction,
    },
    std::{ fs::{ self, File }, io::BufReader, sync::Arc, time::Duration, path::PathBuf },
    tempfile::TempDir,
    tip_router_operator_cli::{
        StakeMetaCollection,
        StakeMeta,
        TipDistributionMeta,
        Delegation,
        GeneratedMerkleTreeCollection as TipRouterMerkleTreeCollection,
        claim_mev_workflow,
        merkle_root_generator_workflow,
        merkle_root_upload_workflow,
    },
    jito_tip_distribution::{self, ID as TIP_DISTRIBUTION_ID},
    jito_tip_payment::{self, ID as TIP_PAYMENT_ID},
    ellipsis_client::EllipsisClient,
    solana_client::rpc_client::RpcClient,
    async_trait::async_trait,
    solana_client::mock_sender::MockSender,
};

struct TestContext {
    context: ProgramTestContext,
    tip_distribution_program_id: Pubkey,
    tip_payment_program_id: Pubkey,
    payer: Keypair,
    stake_account: Keypair,
    vote_account: Keypair,
    temp_dir: TempDir,
    output_dir: PathBuf,
}

struct TestEllipsisClient {
    banks_client: BanksClient,
    payer: Keypair,
    rpc_client: RpcClient,
}

#[async_trait]
impl EllipsisClient for TestEllipsisClient {
    fn get_rpc(&self) -> &RpcClient {
        &self.rpc_client
    }

    fn get_payer(&self) -> &Keypair {
        &self.payer
    }

    async fn send_and_confirm_transaction(&self, transaction: Transaction) -> Result<(), Box<dyn std::error::Error>> {
        self.banks_client.process_transaction(transaction).await?;
        Ok(())
    }
}

impl TestContext {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)?;
    
        let mut program_test = ProgramTest::default();
    
        // Add programs to test environment
        program_test.add_program(
            "jito_tip_distribution",
            TIP_DISTRIBUTION_ID,
            None
        );
        
        program_test.add_program(
            "jito_tip_payment",
            TIP_PAYMENT_ID,
            None
        );
    
        let mut context = program_test.start_with_context().await;
        let payer = Keypair::new();  // Moved up here
        let stake_account = Keypair::new();
        let vote_account = Keypair::new();

        // Fund accounts
        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::transfer(
                    &context.payer.pubkey(),
                    &payer.pubkey(),
                    1_000_000_000
                ),
            ],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash
        );
        context.banks_client.process_transaction(tx).await?;

        // Initialize stake account
        let rent = context.banks_client.get_rent().await?;
        let stake_space = std::mem::size_of::<StakeStateV2>();
        let stake_rent = rent.minimum_balance(stake_space);

        let tx = Transaction::new_signed_with_payer(
            &[
                system_instruction::create_account(
                    &payer.pubkey(),
                    &stake_account.pubkey(),
                    stake_rent,
                    stake_space as u64,
                    &solana_program::stake::program::id()
                ),
            ],
            Some(&payer.pubkey()),
            &[&payer, &stake_account],
            context.last_blockhash
        );
        context.banks_client.process_transaction(tx).await?;

        Ok(Self {
            context,
            tip_distribution_program_id: TIP_DISTRIBUTION_ID,
            tip_payment_program_id: TIP_PAYMENT_ID,
            payer,
            stake_account,
            vote_account,
            temp_dir,
            output_dir,
        })
    }

    fn create_test_stake_meta(&self) -> StakeMetaCollection {
        let stake_meta = StakeMeta {
            validator_vote_account: self.vote_account.pubkey(),
            validator_node_pubkey: self.stake_account.pubkey(),
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                total_tips: 1_000_000,
                merkle_root_upload_authority: self.payer.pubkey(),
                tip_distribution_pubkey: self.tip_distribution_program_id,
                validator_fee_bps: 1000,
            }),
            delegations: vec![Delegation {
                stake_account_pubkey: self.stake_account.pubkey(),
                staker_pubkey: self.payer.pubkey(),
                withdrawer_pubkey: self.payer.pubkey(),
                lamports_delegated: 1_000_000,
            }],
            total_delegated: 1_000_000,
            commission: 10,
        };

        StakeMetaCollection {
            epoch: 0,
            stake_metas: vec![stake_meta],
            bank_hash: "test_bank_hash".to_string(),
            slot: 0,
            tip_distribution_program_id: self.tip_distribution_program_id,
        }
    }

    async fn advance_clock(&mut self, slots: u64) -> Result<(), Box<dyn std::error::Error>> {
        let current_slot = self.context.banks_client.get_root_slot().await?;
        self.context.warp_to_slot(current_slot + slots)?;
        self.context.last_blockhash = self.context.banks_client.get_latest_blockhash().await?;
        Ok(())
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn test_process_epoch() -> Result<(), Box<dyn std::error::Error>> {
    let mut test_context = TestContext::new().await?;

    let previous_epoch_slot = 64;
    test_context.advance_clock(previous_epoch_slot).await?;

    // Create and save stake meta
    let stake_meta = test_context.create_test_stake_meta();
    let stake_meta_path = test_context.output_dir.join("stake-meta.json");
    serde_json::to_writer(File::create(&stake_meta_path)?, &stake_meta)?;

    // Generate merkle trees
    let merkle_tree_path = test_context.output_dir.join("merkle-trees");
    fs::create_dir_all(&merkle_tree_path)?;

    let test_client = TestEllipsisClient {
        banks_client: test_context.context.banks_client.clone(),
        payer: test_context.payer.clone(),
        rpc_client: RpcClient::new_mock_with_mocks("mock".to_string(), vec![]),
    };

    merkle_root_generator_workflow::generate_merkle_root(
        &stake_meta_path,
        &merkle_tree_path,
        &test_client,
    )?;

    // Upload merkle roots
    let keypair_path = test_context.temp_dir.path().join("keypair.json");
    fs::write(&keypair_path, test_context.payer.to_bytes().to_vec())?;

    merkle_root_upload_workflow::upload_merkle_root(
        &merkle_tree_path,
        &keypair_path,
        &test_client,
        &test_context.tip_distribution_program_id,
        5,
        10
    ).await?;

    // Generate meta merkle tree
    // First deserialize as MetaMerkleTreeCollection
    let file = File::open(&merkle_tree_path)?;
    let reader = BufReader::new(file);
    let meta_merkle_trees: MetaMerkleTreeCollection = serde_json::from_reader(reader)?;

    let meta_merkle_tree = MetaMerkleTree::new_from_generated_merkle_tree_collection(
        meta_merkle_trees.clone()
    )?;

    let meta_merkle_path = test_context.output_dir.join("meta-merkle-tree.json");
    meta_merkle_tree.write_to_file(&meta_merkle_path);

    // Convert to TipRouterMerkleTreeCollection for claim_mev_tips
    let tip_router_merkle_trees: TipRouterMerkleTreeCollection = serde_json::from_str(
        &serde_json::to_string(&meta_merkle_trees)?
    )?;

    let claim_result = claim_mev_workflow::claim_mev_tips(
        &tip_router_merkle_trees,
        &test_client,
        test_context.tip_distribution_program_id,
        Arc::new(test_context.payer),
        Duration::from_secs(10),
        1
    ).await;

    assert!(claim_result.is_ok());
    assert!(meta_merkle_path.exists());
    assert!(merkle_tree_path.exists());

    Ok(())
}

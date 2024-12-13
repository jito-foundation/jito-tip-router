pub mod snapshot_creator;

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
        Cli,
        Commands,
        process_epoch,
    },
    jito_tip_distribution::{ self, ID as TIP_DISTRIBUTION_ID },
    jito_tip_payment::{ self, ID as TIP_PAYMENT_ID },
    // ellipsis_client::EllipsisClient,
    solana_client::rpc_client::RpcClient,
    solana_sdk::genesis_config::GenesisConfig,
    self::snapshot_creator::MockSnapshotCreator,
    solana_program::stake::state::StakeState
    // solana_client::mock_sender::MockSender,
};


 struct TestContext {
    pub context: ProgramTestContext,
    pub tip_distribution_program_id: Pubkey,
    pub tip_payment_program_id: Pubkey,
    pub payer: Keypair,
    pub stake_accounts: Vec<Keypair>,  // Changed from single stake_account
    pub vote_account: Keypair,
    pub temp_dir: TempDir,
    pub output_dir: PathBuf,
}

// struct TestEllipsisClient {
//     banks_client: BanksClient,
//     payer: Keypair,
//     rpc_client: RpcClient,
// }

// #[async_trait]
// impl EllipsisClient for TestEllipsisClient {
//     fn get_rpc(&self) -> &RpcClient {
//         &self.rpc_client
//     }

//     fn get_payer(&self) -> &Keypair {
//         &self.payer
//     }

//     async fn send_and_confirm_transaction(&self, transaction: Transaction) -> Result<(), Box<dyn std::error::Error>> {
//         self.banks_client.process_transaction(transaction).await?;
//         Ok(())
//     }
// }

// impl From<TestEllipsisClient> for EllipsisClient {
//     fn from(test_client: TestEllipsisClient) -> Self {
//         EllipsisClient::from_rpc(
//             test_client.rpc_client,
//             &test_client.payer
//         ).expect("Failed to create EllipsisClient from TestEllipsisClient")
//     }
// }

impl TestContext {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let output_dir = temp_dir.path().join("output");
        fs::create_dir_all(&output_dir)?;

        let mut program_test = ProgramTest::default();

        // Add programs to test environment
        program_test.add_program("jito_tip_distribution", TIP_DISTRIBUTION_ID, None);
        program_test.add_program("jito_tip_payment", TIP_PAYMENT_ID, None);

        let mut context = program_test.start_with_context().await;
        let payer = Keypair::new();
        let vote_account = Keypair::new();

        // Fund payer account
        let tx = Transaction::new_signed_with_payer(
            &[system_instruction::transfer(
                &context.payer.pubkey(),
                &payer.pubkey(),
                10_000_000_000, // Increased balance for multiple accounts
            )],
            Some(&context.payer.pubkey()),
            &[&context.payer],
            context.last_blockhash,
        );
        context.banks_client.process_transaction(tx).await?;

        // Create multiple stake accounts
        let stake_accounts = vec![Keypair::new(), Keypair::new(), Keypair::new()];
        
        // Get rent and space requirements
        let rent = context.banks_client.get_rent().await?;
        let stake_space = std::mem::size_of::<StakeStateV2>();
        let stake_rent = rent.minimum_balance(stake_space);

        // Initialize each stake account
        for stake_account in stake_accounts.iter() {
            let tx = Transaction::new_signed_with_payer(
                &[
                    system_instruction::create_account(
                        &payer.pubkey(),
                        &stake_account.pubkey(),
                        stake_rent,
                        stake_space as u64,
                        &solana_program::stake::program::id(),
                    ),
                    solana_program::stake::instruction::initialize(
                        &stake_account.pubkey(),
                        &solana_sdk::stake::state::Authorized {
                            staker: payer.pubkey(),
                            withdrawer: payer.pubkey(),
                        },
                        &solana_sdk::stake::state::Lockup::default(),
                    ),
                ],
                Some(&payer.pubkey()),
                &[&payer, stake_account],
                context.last_blockhash,
            );
            context.banks_client.process_transaction(tx).await?;
            
            // Update blockhash between transactions
            context.last_blockhash = context.banks_client.get_latest_blockhash().await?;
        }

        // Create and initialize vote account (if needed)
        // Add vote account initialization here if required

        Ok(Self {
            context,
            tip_distribution_program_id: TIP_DISTRIBUTION_ID,
            tip_payment_program_id: TIP_PAYMENT_ID,
            payer,
            stake_accounts,  // Store all stake accounts instead of just one
            vote_account,
            temp_dir,
            output_dir,
        })
    }

    fn create_test_stake_meta(&self) -> StakeMetaCollection {
        let stake_meta = StakeMeta {
            validator_vote_account: self.vote_account.pubkey(),
            validator_node_pubkey: self.stake_accounts[0].pubkey(),  // Use first stake account
            maybe_tip_distribution_meta: Some(TipDistributionMeta {
                total_tips: 1_000_000,
                merkle_root_upload_authority: self.payer.pubkey(),
                tip_distribution_pubkey: self.tip_distribution_program_id,
                validator_fee_bps: 1000,
            }),
            delegations: vec![Delegation {
                stake_account_pubkey: self.stake_accounts[0].pubkey(),  // Use first stake account
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
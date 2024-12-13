use anyhow::Result;
use std::{path::PathBuf, fs::{self, File}};
use tip_router_operator_cli::snapshot::SnapshotCreatorTrait;
use solana_program_test::BanksClient;
use solana_sdk::{
    stake::state::StakeState,
    pubkey::Pubkey,
};
use std::sync::Arc;
use tokio::sync::Mutex;  // Changed to tokio's Mutex


pub struct MockSnapshotCreator {
    pub banks_client: Arc<Mutex<BanksClient>>,
    pub output_dir: PathBuf,
}

impl MockSnapshotCreator {
    pub fn new(banks_client: BanksClient, output_dir: PathBuf) -> Self {
        Self { 
            banks_client: Arc::new(Mutex::new(banks_client)),
            output_dir,
        }
    }
}

impl SnapshotCreatorTrait for MockSnapshotCreator {
    async fn create_snapshot(&self, _slot: u64) -> Result<()> {
        let mut banks_client = self.banks_client.lock().await;
        let stake_program_id = solana_sdk::stake::program::id();
        
        // Get all accounts owned by the stake program
        let accounts = banks_client
            .get_account(stake_program_id)
            .await?;

        // For testing, create a dummy stake account state
        let test_stake_pubkey = Pubkey::new_unique();
        let stake_state = StakeState::Initialized(solana_sdk::stake::state::Meta {
            rent_exempt_reserve: 0,
            authorized: solana_sdk::stake::state::Authorized {
                staker: Pubkey::new_unique(),
                withdrawer: Pubkey::new_unique(),
            },
            lockup: solana_sdk::stake::state::Lockup::default(),
        });

        // Create a vector with our test stake account
        let stake_accounts = vec![(test_stake_pubkey, stake_state)];
        
        // Create snapshot directory
        let snapshot_dir = self.output_dir.join("snapshot");
        fs::create_dir_all(&snapshot_dir)?;
        
        // Write stake accounts to file
        let snapshot_file = snapshot_dir.join("stake_accounts.json");
        serde_json::to_writer(
            File::create(snapshot_file)?,
            &stake_accounts
        )?;
        
        Ok(())
    }
}
use jito_priority_fee_distribution_sdk::{
    PriorityFeeDistributionAccount, {self, ClaimStatus},
};
use solana_commitment_config::CommitmentLevel;
use solana_program::pubkey::Pubkey;
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    native_token::{sol_str_to_lamports, LAMPORTS_PER_SOL},
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use solana_system_interface::instruction::transfer;
use solana_vote_interface::{instruction::CreateVoteAccountConfig, state::VoteInit};

use crate::fixtures::TestResult;

pub struct PriorityFeeDistributionClient {
    banks_client: BanksClient,
    payer: Keypair,
}

impl PriorityFeeDistributionClient {
    pub const fn new(banks_client: BanksClient, payer: Keypair) -> Self {
        Self {
            banks_client,
            payer,
        }
    }

    pub async fn process_transaction(&mut self, tx: &Transaction) -> TestResult<()> {
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                tx.clone(),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn airdrop(&mut self, to: &Pubkey, sol: f64) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        let new_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&blockhash)
            .await
            .unwrap();
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(
                        &self.payer.pubkey(),
                        to,
                        sol_str_to_lamports(&sol.to_string()).unwrap(),
                    )],
                    Some(&self.payer.pubkey()),
                    &[&self.payer],
                    new_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn get_priority_fee_distribution_account(
        &mut self,
        vote_account: Pubkey,
        target_epoch: u64,
    ) -> TestResult<PriorityFeeDistributionAccount> {
        let (priority_fee_distribution_address, _) =
            jito_priority_fee_distribution_sdk::derive_priority_fee_distribution_account_address(
                &jito_priority_fee_distribution_sdk::id(),
                &vote_account,
                target_epoch,
            );
        let priority_fee_distribution_account = self
            .banks_client
            .get_account(priority_fee_distribution_address)
            .await?
            .unwrap();
        let mut priority_fee_distribution_data = priority_fee_distribution_account.data.as_slice();
        let priority_fee_distribution =
            PriorityFeeDistributionAccount::deserialize(&mut priority_fee_distribution_data)?;

        Ok(priority_fee_distribution)
    }

    pub async fn get_claim_status_account(
        &mut self,
        claimant: Pubkey,
        priority_fee_distribution_account: Pubkey,
    ) -> TestResult<ClaimStatus> {
        let (claim_status_address, _) =
            jito_priority_fee_distribution_sdk::derive_claim_status_account_address(
                &jito_priority_fee_distribution_sdk::id(),
                &claimant,
                &priority_fee_distribution_account,
            );
        let claim_status_account = self
            .banks_client
            .get_account(claim_status_address)
            .await?
            .unwrap();
        let mut claim_status_data = claim_status_account.data.as_slice();
        let claim_status = ClaimStatus::deserialize(&mut claim_status_data)?;
        Ok(claim_status)
    }

    // Sets up a vote account where the node_pubkey is the payer and the address is a new pubkey
    pub async fn setup_vote_account(&mut self) -> TestResult<Keypair> {
        let vote_keypair = Keypair::new();

        let vote_init = VoteInit {
            node_pubkey: self.payer.pubkey(),
            authorized_voter: self.payer.pubkey(),
            authorized_withdrawer: self.payer.pubkey(),
            commission: 0,
        };

        let ixs = solana_vote_interface::instruction::create_account_with_config(
            &self.payer.pubkey(),
            &vote_keypair.pubkey(),
            &vote_init,
            LAMPORTS_PER_SOL,
            CreateVoteAccountConfig {
                space: solana_vote_interface::state::vote_state_v3::VoteStateV3::size_of() as u64,
                with_seed: None,
            },
        );

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer, &vote_keypair],
            blockhash,
        ))
        .await?;

        Ok(vote_keypair)
    }

    pub async fn do_initialize(&mut self, authority: Pubkey) -> TestResult<()> {
        let (config, bump) = jito_priority_fee_distribution_sdk::derive_config_account_address(
            &jito_priority_fee_distribution_sdk::id(),
        );
        let system_program = solana_system_interface::program::id();
        let initializer = self.payer.pubkey();
        let expired_funds_account = authority;
        let num_epochs_valid = 10;
        let max_validator_commission_bps = 10000;

        self.initialize(
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            config,
            system_program,
            initializer,
            bump,
        )
        .await
    }

    pub async fn initialize(
        &mut self,
        authority: Pubkey,
        expired_funds_account: Pubkey,
        num_epochs_valid: u64,
        max_validator_commission_bps: u16,
        config: Pubkey,
        system_program: Pubkey,
        initializer: Pubkey,
        bump: u8,
    ) -> TestResult<()> {
        let ix = jito_priority_fee_distribution_sdk::instruction::initialize_ix(
            config,
            system_program,
            initializer,
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        );

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_initialize_priority_fee_distribution_account(
        &mut self,
        merkle_root_upload_authority: Pubkey,
        vote_keypair: Keypair,
        epoch: u64,
        validator_commission_bps: u16,
    ) -> TestResult<()> {
        let (config, _) = jito_priority_fee_distribution_sdk::derive_config_account_address(
            &jito_priority_fee_distribution_sdk::id(),
        );
        let system_program = solana_system_interface::program::id();
        let validator_vote_account = vote_keypair.pubkey();
        self.airdrop(&validator_vote_account, 1.0).await?;
        let (priority_fee_distribution_account, account_bump) =
            jito_priority_fee_distribution_sdk::derive_priority_fee_distribution_account_address(
                &jito_priority_fee_distribution_sdk::id(),
                &validator_vote_account,
                epoch,
            );

        self.initialize_priority_fee_distribution_account(
            merkle_root_upload_authority,
            validator_commission_bps,
            config,
            priority_fee_distribution_account,
            system_program,
            validator_vote_account,
            account_bump,
        )
        .await
    }

    pub async fn initialize_priority_fee_distribution_account(
        &mut self,
        merkle_root_upload_authority: Pubkey,
        validator_commission_bps: u16,
        config: Pubkey,
        priority_fee_distribution_account: Pubkey,
        system_program: Pubkey,
        validator_vote_account: Pubkey,
        bump: u8,
    ) -> TestResult<()> {
        let ix =
            jito_priority_fee_distribution_sdk::instruction::initialize_priority_fee_distribution_account_ix(
                config,
                priority_fee_distribution_account,
                system_program,
                validator_vote_account,
                self.payer.pubkey(),
                merkle_root_upload_authority,
                validator_commission_bps,
                bump,
            );

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    #[allow(dead_code)]
    pub async fn do_claim(
        &mut self,
        proof: Vec<[u8; 32]>,
        amount: u64,
        claimant: Pubkey,
        epoch: u64,
        merkle_root_upload_authority: Pubkey,
    ) -> TestResult<()> {
        let (config, _) = jito_priority_fee_distribution_sdk::derive_config_account_address(
            &jito_priority_fee_distribution_sdk::id(),
        );
        let system_program = solana_system_interface::program::id();
        let (priority_fee_distribution_account, _) =
            jito_priority_fee_distribution_sdk::derive_priority_fee_distribution_account_address(
                &jito_priority_fee_distribution_sdk::id(),
                &claimant,
                epoch,
            );
        let (claim_status, claim_status_bump) =
            jito_priority_fee_distribution_sdk::derive_claim_status_account_address(
                &jito_priority_fee_distribution_sdk::id(),
                &claimant,
                &priority_fee_distribution_account,
            );
        let payer = self.payer.pubkey();

        self.claim(
            proof,
            amount,
            config,
            priority_fee_distribution_account,
            merkle_root_upload_authority,
            claim_status,
            claimant,
            payer,
            system_program,
            claim_status_bump,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn claim(
        &mut self,
        proof: Vec<[u8; 32]>,
        amount: u64,
        config: Pubkey,
        priority_fee_distribution_account: Pubkey,
        merkle_root_upload_authority: Pubkey,
        claim_status: Pubkey,
        claimant: Pubkey,
        payer: Pubkey,
        system_program: Pubkey,
        bump: u8,
    ) -> TestResult<()> {
        let ix = jito_priority_fee_distribution_sdk::instruction::claim_ix(
            config,
            priority_fee_distribution_account,
            merkle_root_upload_authority,
            claim_status,
            claimant,
            payer,
            system_program,
            proof,
            amount,
            bump,
        );

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }
}

// TODO write this

// Import tip distribution program

// Basic methods for initializing the joint
// Remember the merkle_root_upload_authority system may be changing a bit

// Getters for the Tip Distribution account to verify that we've set the merkle root correctly
use solana_program::{pubkey::Pubkey, system_instruction::transfer};
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    native_token::sol_to_lamports,
    signature::{Keypair, Signer},
    transaction::Transaction,
};

use crate::fixtures::{TestError, TestResult};

pub struct TipDistributionClient {
    banks_client: BanksClient,
    payer: Keypair,
}

impl TipDistributionClient {
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
                    &[transfer(&self.payer.pubkey(), to, sol_to_lamports(sol))],
                    Some(&self.payer.pubkey()),
                    &[&self.payer],
                    new_blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }
    //
    pub async fn setup_vote_account(&mut self) -> TestResult<Pubkey> {
        // TODO: new keypair, invoke vote program??
        let vote_account_keypair = Keypair::new();

        Ok(vote_account_keypair.pubkey())
    }

    pub async fn do_initialize(&mut self, authority: Pubkey) -> TestResult<()> {
        let (config, bump) =
            jito_tip_distribution_sdk::derive_config_account_address(&jito_tip_distribution::id());
        let system_program = solana_program::system_program::id();
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
        let ix = jito_tip_distribution_sdk::instruction::initialize_ix(
            jito_tip_distribution::id(),
            jito_tip_distribution_sdk::instruction::InitializeArgs {
                authority,
                expired_funds_account,
                num_epochs_valid,
                max_validator_commission_bps,
                bump,
            },
            jito_tip_distribution_sdk::instruction::InitializeAccounts {
                config,
                system_program,
                initializer,
            },
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

    pub async fn do_initialize_tip_distribution_account(
        &mut self,
        merkle_root_upload_authority: Pubkey,
        validator_vote_account: Pubkey,
        epoch: u64,
        validator_commission_bps: u16,
    ) -> TestResult<()> {
        let (config, _) =
            jito_tip_distribution_sdk::derive_config_account_address(&jito_tip_distribution::id());
        let system_program = solana_program::system_program::id();
        let (tip_distribution_account, account_bump) =
            jito_tip_distribution_sdk::derive_tip_distribution_account_address(
                &jito_tip_distribution::id(),
                &validator_vote_account,
                epoch,
            );
        let signer = self.payer.pubkey();

        self.initialize_tip_distribution_account(
            merkle_root_upload_authority,
            validator_commission_bps,
            config,
            tip_distribution_account,
            system_program,
            validator_vote_account,
            signer,
            account_bump,
        )
        .await
    }

    pub async fn initialize_tip_distribution_account(
        &mut self,
        merkle_root_upload_authority: Pubkey,
        validator_commission_bps: u16,
        config: Pubkey,
        tip_distribution_account: Pubkey,
        system_program: Pubkey,
        validator_vote_account: Pubkey,
        signer: Pubkey,
        bump: u8,
    ) -> TestResult<()> {
        let ix = jito_tip_distribution_sdk::instruction::initialize_tip_distribution_account_ix(
            jito_tip_distribution::id(),
            jito_tip_distribution_sdk::instruction::InitializeTipDistributionAccountArgs {
                merkle_root_upload_authority,
                validator_commission_bps,
                bump,
            },
            jito_tip_distribution_sdk::instruction::InitializeTipDistributionAccountAccounts {
                config,
                tip_distribution_account,
                system_program,
                validator_vote_account,
                signer,
            },
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

    pub async fn do_claim(
        &mut self,
        proof: Vec<[u8; 32]>,
        amount: u64,
        claimant: Pubkey,
    ) -> TestResult<()> {
        let (config, _) =
            jito_tip_distribution_sdk::derive_config_account_address(&jito_tip_distribution::id());
        let system_program = solana_program::system_program::id();
        let (tip_distribution_account, _) =
            jito_tip_distribution_sdk::derive_tip_distribution_account_address(
                &jito_tip_distribution::id(),
                &claimant,
                0, // Assuming epoch is 0 for simplicity
            );
        let (claim_status, claim_status_bump) = Pubkey::find_program_address(
            &[
                jito_tip_distribution::state::ClaimStatus::SEED,
                claimant.as_ref(),
                tip_distribution_account.as_ref(),
            ],
            &jito_tip_distribution::id(),
        );
        let payer = self.payer.pubkey();

        self.claim(
            proof,
            amount,
            config,
            tip_distribution_account,
            claim_status,
            claimant,
            payer,
            system_program,
            claim_status_bump,
        )
        .await
    }

    pub async fn claim(
        &mut self,
        proof: Vec<[u8; 32]>,
        amount: u64,
        config: Pubkey,
        tip_distribution_account: Pubkey,
        claim_status: Pubkey,
        claimant: Pubkey,
        payer: Pubkey,
        system_program: Pubkey,
        bump: u8,
    ) -> TestResult<()> {
        let ix = jito_tip_distribution_sdk::instruction::claim_ix(
            jito_tip_distribution::id(),
            jito_tip_distribution_sdk::instruction::ClaimArgs {
                proof,
                amount,
                bump,
            },
            jito_tip_distribution_sdk::instruction::ClaimAccounts {
                config,
                tip_distribution_account,
                claim_status,
                claimant,
                payer,
                system_program,
            },
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

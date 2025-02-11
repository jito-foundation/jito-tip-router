#![allow(deprecated)] // using deprecated borsh to align with mainnet stake pool version
use jito_tip_router_core::constants::JITOSOL_MINT;
use solana_program::{
    borsh1::{get_instance_packed_len, get_packed_len},
    pubkey::Pubkey,
    stake,
};
use solana_program_test::BanksClient;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signer},
    system_instruction,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use spl_stake_pool::{
    find_withdraw_authority_program_address, instruction,
    state::{Fee, StakePool, ValidatorList},
};

use crate::fixtures::TestResult;

// Constants
const STAKE_STATE_LEN: usize = 200;
const MINIMUM_RESERVE_LAMPORTS: u64 = 1_000_000_000;

pub struct StakePoolClient {
    banks_client: BanksClient,
    payer: Keypair,
    stake_pool_keypair: Keypair,
}

#[derive(Debug, Clone, Copy)]
pub struct PoolRoot {
    pub pool_address: Pubkey,
    pub pool_mint: Pubkey,
    pub reserve_stake: Pubkey,
    pub manager_fee_account: Pubkey,
    pub referrer_pool_tokens_account: Pubkey,
    pub withdraw_authority: Pubkey,
    pub validator_list: Pubkey,
}

impl StakePoolClient {
    pub const fn new(
        banks_client: BanksClient,
        payer: Keypair,
        stake_pool_keypair: Keypair,
    ) -> Self {
        Self {
            banks_client,
            payer,
            stake_pool_keypair,
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

    pub async fn do_initialize_stake_pool(&mut self) -> TestResult<PoolRoot> {
        let fee = Fee {
            numerator: 0,
            denominator: 1,
        };

        let withdrawal_fee = Fee {
            numerator: 0,
            denominator: 1,
        };

        let deposit_fee = Fee {
            numerator: 0,
            denominator: 1,
        };

        let referral_fee = 0;
        let max_validators = 1;

        let payer = self.payer.insecure_clone();

        self.initialize_stake_pool(
            &payer,
            &payer.pubkey(),
            fee,
            withdrawal_fee,
            deposit_fee,
            referral_fee,
            max_validators,
        )
        .await
    }

    pub async fn initialize_stake_pool(
        &mut self,
        manager: &Keypair,
        staker: &Pubkey,
        fee: Fee,
        withdrawal_fee: Fee,
        deposit_fee: Fee,
        referral_fee: u8,
        max_validators: u32,
    ) -> TestResult<PoolRoot> {
        let stake_pool = self.stake_pool_keypair.insecure_clone();
        let validator_list = Keypair::new();
        let pool_mint = JITOSOL_MINT;
        let reserve_stake = Keypair::new();
        let manager_fee_account = get_associated_token_address(&manager.pubkey(), &pool_mint);
        let referrer_pool_tokens_account = Keypair::new();

        let withdraw_authority =
            find_withdraw_authority_program_address(&spl_stake_pool::id(), &stake_pool.pubkey()).0;

        let reserve_stake_ix = vec![
            system_instruction::create_account(
                &self.payer.pubkey(),
                &reserve_stake.pubkey(),
                MINIMUM_RESERVE_LAMPORTS,
                STAKE_STATE_LEN as u64,
                &stake::program::id(),
            ),
            stake::instruction::initialize(
                &reserve_stake.pubkey(),
                &stake::state::Authorized {
                    staker: withdraw_authority,
                    withdrawer: withdraw_authority,
                },
                &stake::state::Lockup::default(),
            ),
        ];

        let manager_fee_account_ix =
            spl_associated_token_account::instruction::create_associated_token_account_idempotent(
                &self.payer.pubkey(),
                &manager.pubkey(),
                &pool_mint,
                &spl_token::id(),
            );

        let validator_list_size = get_instance_packed_len(&ValidatorList::new(max_validators))?;
        let create_validator_list_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &validator_list.pubkey(),
            self.banks_client
                .get_rent()
                .await?
                .minimum_balance(validator_list_size),
            validator_list_size as u64,
            &spl_stake_pool::id(),
        );

        let create_pool_ix = system_instruction::create_account(
            &self.payer.pubkey(),
            &stake_pool.pubkey(),
            self.banks_client
                .get_rent()
                .await?
                .minimum_balance(get_packed_len::<StakePool>()),
            get_packed_len::<StakePool>() as u64,
            &spl_stake_pool::id(),
        );

        let init_pool_ix = instruction::initialize(
            &spl_stake_pool::id(),
            &stake_pool.pubkey(),
            &manager.pubkey(),
            staker,
            &withdraw_authority,
            &validator_list.pubkey(),
            &reserve_stake.pubkey(),
            &pool_mint,
            &manager_fee_account,
            &spl_token::id(),
            None,
            fee,
            withdrawal_fee,
            deposit_fee,
            referral_fee,
            max_validators,
        );

        let blockhash = self.banks_client.get_latest_blockhash().await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[reserve_stake_ix, vec![manager_fee_account_ix]].concat(),
            Some(&self.payer.pubkey()),
            &[&self.payer, &reserve_stake],
            blockhash,
        ))
        .await?;

        self.process_transaction(&Transaction::new_signed_with_payer(
            &[create_validator_list_ix, create_pool_ix, init_pool_ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, &validator_list, &stake_pool, manager],
            blockhash,
        ))
        .await?;

        Ok(PoolRoot {
            pool_address: stake_pool.pubkey(),
            pool_mint,
            reserve_stake: reserve_stake.pubkey(),
            manager_fee_account,
            referrer_pool_tokens_account: referrer_pool_tokens_account.pubkey(),
            withdraw_authority,
            validator_list: validator_list.pubkey(),
        })
    }

    pub async fn update_stake_pool_balance(&mut self, pool_root: &PoolRoot) -> TestResult<()> {
        let ix = instruction::update_stake_pool_balance(
            &spl_stake_pool::id(),
            &pool_root.pool_address,
            &pool_root.withdraw_authority,
            &pool_root.validator_list,
            &pool_root.reserve_stake,
            &pool_root.manager_fee_account,
            &pool_root.pool_mint,
            &spl_token::id(),
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

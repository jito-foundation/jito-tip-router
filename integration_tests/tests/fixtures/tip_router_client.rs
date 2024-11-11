use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::config::Config;
use jito_tip_router_client::{
    instructions::{
        AdminUpdateWeightTableBuilder, InitializeEpochSnapshotBuilder, InitializeNCNConfigBuilder,
        InitializeWeightTableBuilder, SetConfigFeesBuilder, SetNewAdminBuilder,
    },
    types::ConfigAdminRole,
};
use jito_tip_router_core::{
    epoch_snapshot::EpochSnapshot, error::TipRouterError, ncn_config::NcnConfig,
    weight_table::WeightTable,
};
use solana_program::{
    instruction::InstructionError, native_token::sol_to_lamports, pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_program_test::BanksClient;
use solana_sdk::{
    commitment_config::CommitmentLevel,
    signature::{Keypair, Signer},
    system_program,
    transaction::{Transaction, TransactionError},
};

use super::restaking_client::NcnRoot;
use crate::fixtures::{TestError, TestResult};

pub struct TipRouterClient {
    banks_client: BanksClient,
    payer: Keypair,
}

impl TipRouterClient {
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
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(&self.payer.pubkey(), to, sol_to_lamports(sol))],
                    Some(&self.payer.pubkey()),
                    &[&self.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn get_restaking_config(&mut self) -> TestResult<Config> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let restaking_config_data = self
            .banks_client
            .get_account(restaking_config)
            .await?
            .unwrap();
        Ok(*Config::try_from_slice_unchecked(restaking_config_data.data.as_slice()).unwrap())
    }

    pub async fn get_ncn_config(&mut self, ncn_pubkey: Pubkey) -> TestResult<NcnConfig> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_pubkey).0;
        let config = self.banks_client.get_account(config_pda).await?.unwrap();
        Ok(*NcnConfig::try_from_slice_unchecked(config.data.as_slice()).unwrap())
    }

    pub async fn do_initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
    ) -> TestResult<()> {
        self.airdrop(&self.payer.pubkey(), 1.0).await?;

        let ncn_admin_pubkey = ncn_admin.pubkey();
        self.initialize_config(ncn, ncn_admin, ncn_admin_pubkey, ncn_admin_pubkey, 0, 0, 0)
            .await
    }

    pub async fn initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
        fee_wallet: Pubkey,
        tie_breaker_admin: Pubkey,
        dao_fee_bps: u64,
        ncn_fee_bps: u64,
        block_engine_fee_bps: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = InitializeNCNConfigBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .ncn_admin(ncn_admin.pubkey())
            .fee_wallet(fee_wallet)
            .tie_breaker_admin(tie_breaker_admin)
            .restaking_program_id(jito_restaking_program::id())
            .dao_fee_bps(dao_fee_bps)
            .ncn_fee_bps(ncn_fee_bps)
            .block_engine_fee_bps(block_engine_fee_bps)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&ncn_admin.pubkey()),
            &[&ncn_admin],
            blockhash,
        ))
        .await
    }

    pub async fn do_set_config_fees(
        &mut self,
        dao_fee_bps: u64,
        ncn_fee_bps: u64,
        block_engine_fee_bps: u64,
        fee_wallet: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_root.ncn_pubkey).0;
        self.airdrop(&ncn_root.ncn_admin.pubkey(), 1.0).await?;
        self.set_config_fees(
            config_pda,
            dao_fee_bps,
            ncn_fee_bps,
            block_engine_fee_bps,
            fee_wallet,
            &ncn_root,
        )
        .await
    }

    pub async fn set_config_fees(
        &mut self,
        config_pda: Pubkey,
        dao_fee_bps: u64,
        ncn_fee_bps: u64,
        block_engine_fee_bps: u64,
        fee_wallet: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let ix = SetConfigFeesBuilder::new()
            .restaking_config(restaking_config)
            .config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey())
            .restaking_program_id(jito_restaking_program::id())
            .new_dao_fee_bps(dao_fee_bps)
            .new_ncn_fee_bps(ncn_fee_bps)
            .new_block_engine_fee_bps(block_engine_fee_bps)
            .new_fee_wallet(fee_wallet)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&ncn_root.ncn_admin.pubkey()),
            &[&ncn_root.ncn_admin],
            blockhash,
        ))
        .await
    }

    pub async fn do_set_new_admin(
        &mut self,
        role: ConfigAdminRole,
        new_admin: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_root.ncn_pubkey).0;
        self.airdrop(&ncn_root.ncn_admin.pubkey(), 1.0).await?;
        self.set_new_admin(config_pda, role, new_admin, ncn_root)
            .await
    }

    pub async fn set_new_admin(
        &mut self,
        config_pda: Pubkey,
        role: ConfigAdminRole,
        new_admin: Pubkey,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let ix = SetNewAdminBuilder::new()
            .config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey())
            .new_admin(new_admin)
            .restaking_program_id(jito_restaking_program::id())
            .role(role)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&ncn_root.ncn_admin.pubkey()),
            &[&ncn_root.ncn_admin],
            blockhash,
        ))
        .await
    }

    pub async fn do_initialize_weight_table(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
    ) -> TestResult<()> {
        self.initialize_weight_table(ncn, current_slot).await
    }

    pub async fn initialize_weight_table(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = current_slot / restaking_config_account.epoch_length();

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = InitializeWeightTableBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(config_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .payer(self.payer.pubkey())
            .restaking_program_id(jito_restaking_program::id())
            .system_program(system_program::id())
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_admin_update_weight_table(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
        mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        self.admin_update_weight_table(ncn, current_slot, mint, weight)
            .await
    }

    pub async fn admin_update_weight_table(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
        mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = current_slot / restaking_config_account.epoch_length();

        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = AdminUpdateWeightTableBuilder::new()
            .ncn(ncn)
            .weight_table(weight_table)
            .weight_table_admin(self.payer.pubkey())
            .mint(mint)
            .restaking_program_id(jito_restaking_program::id())
            .weight(weight)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_initialize_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
    ) -> TestResult<()> {
        self.initialize_epoch_snapshot(ncn, current_slot).await
    }

    pub async fn initialize_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        current_slot: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = current_slot / restaking_config_account.epoch_length();

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = InitializeEpochSnapshotBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(config_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .payer(self.payer.pubkey())
            .restaking_program_id(jito_restaking_program::id())
            .system_program(system_program::id())
            .instruction();

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

#[inline(always)]
#[track_caller]
pub fn assert_tip_router_error<T>(
    test_error: Result<T, TestError>,
    tip_router_error: TipRouterError,
) {
    assert!(test_error.is_err());
    assert_eq!(
        test_error.err().unwrap().to_transaction_error().unwrap(),
        TransactionError::InstructionError(0, InstructionError::Custom(tip_router_error as u32))
    );
}

use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn_operator_state::NcnOperatorState, ncn_vault_ticket::NcnVaultTicket,
};
use jito_tip_router_client::{
    instructions::{
        AdminUpdateWeightTableBuilder, InitializeEpochSnapshotBuilder, InitializeNCNConfigBuilder,
        InitializeOperatorSnapshotBuilder, InitializeTrackedMintsBuilder,
        InitializeWeightTableBuilder, RegisterMintBuilder, SetConfigFeesBuilder,
        SetNewAdminBuilder, SnapshotVaultOperatorDelegationBuilder,
    },
    types::ConfigAdminRole,
};
use jito_tip_router_core::{
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    error::TipRouterError,
    ncn_config::NcnConfig,
    tracked_mints::TrackedMints,
    weight_table::WeightTable,
};
use jito_vault_core::{
    vault_ncn_ticket::VaultNcnTicket, vault_operator_delegation::VaultOperatorDelegation,
};
use solana_program::{
    instruction::InstructionError, native_token::sol_to_lamports, pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    clock::Clock,
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

    pub async fn setup_tip_router(&mut self, ncn_root: &NcnRoot) -> TestResult<()> {
        self.do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;
        self.do_initialize_tracked_mints(ncn_root.ncn_pubkey)
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

    pub async fn get_tracked_mints(&mut self, ncn_pubkey: Pubkey) -> TestResult<TrackedMints> {
        let tracked_mints_pda =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn_pubkey).0;
        let tracked_mints = self
            .banks_client
            .get_account(tracked_mints_pda)
            .await?
            .unwrap();
        Ok(*TrackedMints::try_from_slice_unchecked(tracked_mints.data.as_slice()).unwrap())
    }

    #[allow(dead_code)]
    pub async fn get_weight_table(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<WeightTable> {
        let address =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account = WeightTable::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();

        Ok(*account)
    }

    pub async fn get_epoch_snapshot(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<EpochSnapshot> {
        let address =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account = EpochSnapshot::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();

        Ok(*account)
    }

    #[allow(dead_code)]
    pub async fn get_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<OperatorSnapshot> {
        let address = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            ncn_epoch,
        )
        .0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account =
            OperatorSnapshot::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();

        Ok(*account)
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
            .restaking_program(jito_restaking_program::id())
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
            .restaking_program(jito_restaking_program::id())
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
            .restaking_program(jito_restaking_program::id())
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

        let tracked_mints_pda =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = InitializeWeightTableBuilder::new()
            .restaking_config(restaking_config)
            .tracked_mints(tracked_mints_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
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
            .restaking_program(jito_restaking_program::id())
            .weight(weight)
            .ncn_epoch(ncn_epoch)
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

    pub async fn do_initialize_tracked_mints(&mut self, ncn: Pubkey) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        self.initialize_tracked_mints(&ncn_config, &tracked_mints, &ncn)
            .await
    }

    pub async fn initialize_tracked_mints(
        &mut self,
        ncn_config: &Pubkey,
        tracked_mints: &Pubkey,
        ncn: &Pubkey,
    ) -> TestResult<()> {
        let ix = InitializeTrackedMintsBuilder::new()
            .ncn_config(*ncn_config)
            .tracked_mints(*tracked_mints)
            .ncn(*ncn)
            .payer(self.payer.pubkey())
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

    pub async fn do_register_mint(
        &mut self,
        ncn: Pubkey,
        vault: Pubkey,
        vault_ncn_ticket: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let restaking_config_address =
            Config::find_program_address(&jito_restaking_program::id()).0;
        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let restaking_config = self.get_restaking_config().await?;
        let current_slot = self.banks_client.get_sysvar::<Clock>().await?.slot;
        let ncn_epoch = current_slot / restaking_config.epoch_length();
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        self.register_mint(
            restaking_config_address,
            tracked_mints,
            ncn,
            weight_table,
            vault,
            vault_ncn_ticket,
            ncn_vault_ticket,
        )
        .await
    }

    pub async fn register_mint(
        &mut self,
        restaking_config: Pubkey,
        tracked_mints: Pubkey,
        ncn: Pubkey,
        weight_table: Pubkey,
        vault: Pubkey,
        vault_ncn_ticket: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let ix = RegisterMintBuilder::new()
            .restaking_config(restaking_config)
            .tracked_mints(tracked_mints)
            .ncn(ncn)
            .weight_table(weight_table)
            .vault(vault)
            .vault_ncn_ticket(vault_ncn_ticket)
            .ncn_vault_ticket(ncn_vault_ticket)
            .restaking_program_id(jito_restaking_program::id())
            .vault_program_id(jito_vault_program::id())
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

    pub async fn do_initialize_epoch_snapshot(&mut self, ncn: Pubkey, slot: u64) -> TestResult<()> {
        self.initialize_epoch_snapshot(ncn, slot).await
    }

    pub async fn initialize_epoch_snapshot(&mut self, ncn: Pubkey, slot: u64) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = InitializeEpochSnapshotBuilder::new()
            .ncn_config(config_pda)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .tracked_mints(tracked_mints)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .first_slot_of_ncn_epoch(slot)
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

    pub async fn do_initalize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        slot: u64,
    ) -> TestResult<()> {
        self.initalize_operator_snapshot(operator, ncn, slot).await
    }

    pub async fn initalize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        slot: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ncn_operator_state =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), &ncn, &operator)
                .0;

        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            ncn_epoch,
        )
        .0;

        let ix = InitializeOperatorSnapshotBuilder::new()
            .ncn_config(config_pda)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .first_slot_of_ncn_epoch(slot)
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

    pub async fn do_snapshot_vault_operator_delegation(
        &mut self,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        slot: u64,
    ) -> TestResult<()> {
        self.snapshot_vault_operator_delegation(vault, operator, ncn, slot)
            .await
    }

    pub async fn snapshot_vault_operator_delegation(
        &mut self,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        slot: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let restaking_config_account = self.get_restaking_config().await?;
        let ncn_epoch = slot / restaking_config_account.epoch_length();

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            ncn_epoch,
        )
        .0;

        let vault_ncn_ticket =
            VaultNcnTicket::find_program_address(&jito_vault_program::id(), &vault, &ncn).0;

        let ncn_vault_ticket =
            NcnVaultTicket::find_program_address(&jito_restaking_program::id(), &ncn, &vault).0;

        let vault_operator_delegation = VaultOperatorDelegation::find_program_address(
            &jito_vault_program::id(),
            &vault,
            &operator,
        )
        .0;

        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch).0;

        let ix = SnapshotVaultOperatorDelegationBuilder::new()
            .ncn_config(config_pda)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .vault(vault)
            .vault_ncn_ticket(vault_ncn_ticket)
            .ncn_vault_ticket(ncn_vault_ticket)
            .vault_operator_delegation(vault_operator_delegation)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .vault_program(jito_vault_program::id())
            .restaking_program(jito_restaking_program::id())
            .first_slot_of_ncn_epoch(slot)
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

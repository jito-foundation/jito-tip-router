use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn_operator_state::NcnOperatorState, ncn_vault_ticket::NcnVaultTicket,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, jito_tip_distribution};
use jito_tip_router_client::{
    instructions::{
        AdminUpdateWeightTableBuilder, CastVoteBuilder, DistributeBaseNcnRewardRouteBuilder,
        DistributeBaseRewardsBuilder, DistributeNcnOperatorRewardsBuilder,
        DistributeNcnVaultRewardsBuilder, InitializeBallotBoxBuilder,
        InitializeBaseRewardRouterBuilder, InitializeEpochSnapshotBuilder,
        InitializeNCNConfigBuilder, InitializeNcnRewardRouterBuilder,
        InitializeOperatorSnapshotBuilder, InitializeTrackedMintsBuilder,
        InitializeWeightTableBuilder, ReallocBallotBoxBuilder, ReallocBaseRewardRouterBuilder,
        ReallocOperatorSnapshotBuilder, ReallocWeightTableBuilder, RegisterMintBuilder,
        RouteBaseRewardsBuilder, RouteNcnRewardsBuilder, SetConfigFeesBuilder,
        SetMerkleRootBuilder, SetNewAdminBuilder, SetTieBreakerBuilder,
        SetTrackedMintNcnFeeGroupBuilder, SnapshotVaultOperatorDelegationBuilder,
    },
    types::ConfigAdminRole,
};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_fee_group::BaseFeeGroup,
    base_reward_router::BaseRewardRouter,
    constants::MAX_REALLOC_BYTES,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    error::TipRouterError,
    ncn_config::NcnConfig,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::NcnRewardRouter,
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
    compute_budget::ComputeBudgetInstruction,
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

    pub async fn get_ballot_box(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<BallotBox> {
        let address =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let raw_account = self.banks_client.get_account(address).await?.unwrap();
        Ok(*BallotBox::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
    }

    pub async fn get_base_reward_router(
        &mut self,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<BaseRewardRouter> {
        let address =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, ncn_epoch)
                .0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account =
            BaseRewardRouter::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();
        Ok(*account)
    }

    pub async fn get_ncn_reward_router(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_epoch: u64,
    ) -> TestResult<NcnRewardRouter> {
        let address = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            ncn_epoch,
        )
        .0;

        let raw_account = self.banks_client.get_account(address).await?.unwrap();

        let account =
            NcnRewardRouter::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap();

        Ok(*account)
    }

    pub async fn do_initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
    ) -> TestResult<()> {
        self.airdrop(&self.payer.pubkey(), 1.0).await?;

        let ncn_admin_pubkey = ncn_admin.pubkey();
        self.initialize_config(
            ncn,
            ncn_admin,
            &ncn_admin_pubkey,
            &ncn_admin_pubkey,
            0,
            0,
            0,
        )
        .await
    }

    pub async fn initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
        tie_breaker_admin: &Pubkey,
        fee_wallet: &Pubkey,
        block_engine_fee_bps: u16,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = InitializeNCNConfigBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .ncn_admin(ncn_admin.pubkey())
            .fee_wallet(*fee_wallet)
            .tie_breaker_admin(*tie_breaker_admin)
            .restaking_program(jito_restaking_program::id())
            .dao_fee_bps(dao_fee_bps)
            .default_ncn_fee_bps(default_ncn_fee_bps)
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
        new_block_engine_fee_bps: Option<u16>,
        base_fee_group: Option<BaseFeeGroup>,
        new_base_fee_wallet: Option<Pubkey>,
        new_base_fee_bps: Option<u16>,
        ncn_fee_group: Option<NcnFeeGroup>,
        new_ncn_fee_bps: Option<u16>,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_root.ncn_pubkey).0;
        self.airdrop(&ncn_root.ncn_admin.pubkey(), 1.0).await?;
        self.set_config_fees(
            config_pda,
            new_block_engine_fee_bps,
            base_fee_group,
            new_base_fee_wallet,
            new_base_fee_bps,
            ncn_fee_group,
            new_ncn_fee_bps,
            ncn_root,
        )
        .await
    }

    pub async fn set_config_fees(
        &mut self,
        config_pda: Pubkey,
        new_block_engine_fee_bps: Option<u16>,
        base_fee_group: Option<BaseFeeGroup>,
        new_base_fee_wallet: Option<Pubkey>,
        new_base_fee_bps: Option<u16>,
        ncn_fee_group: Option<NcnFeeGroup>,
        new_ncn_fee_bps: Option<u16>,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let ix = {
            let mut builder = SetConfigFeesBuilder::new();
            builder
                .restaking_config(restaking_config)
                .config(config_pda)
                .ncn(ncn_root.ncn_pubkey)
                .ncn_admin(ncn_root.ncn_admin.pubkey())
                .restaking_program(jito_restaking_program::id());

            if let Some(new_block_engine_fee_bps) = new_block_engine_fee_bps {
                builder.new_block_engine_fee_bps(new_block_engine_fee_bps);
            }

            if let Some(base_fee_group) = base_fee_group {
                builder.base_fee_group(base_fee_group.group);
            }

            if let Some(new_base_fee_wallet) = new_base_fee_wallet {
                builder.new_base_fee_wallet(new_base_fee_wallet);
            }

            if let Some(new_base_fee_bps) = new_base_fee_bps {
                builder.new_base_fee_bps(new_base_fee_bps);
            }

            if let Some(ncn_fee_group) = ncn_fee_group {
                builder.ncn_fee_group(ncn_fee_group.group);
            }

            if let Some(new_ncn_fee_bps) = new_ncn_fee_bps {
                builder.new_ncn_fee_bps(new_ncn_fee_bps);
            }

            builder.instruction()
        };

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

    pub async fn do_full_initialize_weight_table(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_weight_table(ncn, epoch).await?;
        let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_weight_table(ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    pub async fn do_initialize_weight_table(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.initialize_weight_table(ncn, epoch).await
    }

    pub async fn initialize_weight_table(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let tracked_mints_pda =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = InitializeWeightTableBuilder::new()
            .restaking_config(restaking_config)
            .tracked_mints(tracked_mints_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .epoch(epoch)
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
        epoch: u64,
        mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        self.admin_update_weight_table(ncn, epoch, mint, weight)
            .await
    }

    pub async fn admin_update_weight_table(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = AdminUpdateWeightTableBuilder::new()
            .ncn(ncn)
            .weight_table(weight_table)
            .weight_table_admin(self.payer.pubkey())
            .mint(mint)
            .restaking_program(jito_restaking_program::id())
            .weight(weight)
            .ncn_epoch(epoch)
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

        let epoch = self.banks_client.get_sysvar::<Clock>().await?.epoch;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

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

    pub async fn do_set_tracked_mint_ncn_fee_group(
        &mut self,
        ncn: Pubkey,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
        epoch: u64,
    ) -> TestResult<()> {
        self.set_tracked_mint_ncn_fee_group(ncn, vault_index, ncn_fee_group, epoch)
            .await
    }

    pub async fn set_tracked_mint_ncn_fee_group(
        &mut self,
        ncn: Pubkey,
        vault_index: u64,
        ncn_fee_group: NcnFeeGroup,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        //TODO: Check admin is correct
        let admin = self.payer.pubkey();

        let ix = SetTrackedMintNcnFeeGroupBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .weight_table(weight_table)
            .tracked_mints(tracked_mints)
            .admin(admin)
            .restaking_program(jito_restaking_program::id())
            .vault_index(vault_index)
            .ncn_fee_group(ncn_fee_group.group)
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
        epoch: u64,
    ) -> TestResult<()> {
        self.initialize_epoch_snapshot(ncn, epoch).await
    }

    pub async fn initialize_epoch_snapshot(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

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
            .epoch(epoch)
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

    pub async fn do_full_initialize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_operator_snapshot(operator, ncn, epoch)
            .await?;
        let num_reallocs =
            (OperatorSnapshot::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_operator_snapshot(operator, ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    pub async fn do_initialize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.initialize_operator_snapshot(operator, ncn, epoch)
            .await
    }

    pub async fn initialize_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let ncn_operator_state =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), &ncn, &operator)
                .0;
        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
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
            .epoch(epoch)
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
        epoch: u64,
    ) -> TestResult<()> {
        self.snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
            .await
    }

    pub async fn snapshot_vault_operator_delegation(
        &mut self,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
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
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;

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
            .tracked_mints(tracked_mints)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .vault_program(jito_vault_program::id())
            .restaking_program(jito_restaking_program::id())
            .epoch(epoch)
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

    pub async fn do_full_initialize_ballot_box(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_ballot_box(ncn, epoch).await?;
        let num_reallocs = (BallotBox::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_ballot_box(ncn, epoch, num_reallocs).await?;
        Ok(())
    }

    pub async fn do_initialize_ballot_box(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> Result<(), TestError> {
        let ncn_config = jito_tip_router_core::ncn_config::NcnConfig::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
        )
        .0;

        let ballot_box = jito_tip_router_core::ballot_box::BallotBox::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
            epoch,
        )
        .0;

        self.initialize_ballot_box(ncn_config, ballot_box, ncn, epoch)
            .await
    }

    pub async fn initialize_ballot_box(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> Result<(), TestError> {
        let ix = InitializeBallotBoxBuilder::new()
            .ncn_config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .payer(self.payer.pubkey())
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

    pub async fn do_realloc_ballot_box(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ncn_config = jito_tip_router_core::ncn_config::NcnConfig::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
        )
        .0;

        let ballot_box = jito_tip_router_core::ballot_box::BallotBox::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
            epoch,
        )
        .0;

        self.realloc_ballot_box(ncn_config, ballot_box, ncn, epoch, num_reallocations)
            .await
    }

    pub async fn realloc_ballot_box(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ix = ReallocBallotBoxBuilder::new()
            .ncn_config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .payer(self.payer.pubkey())
            .instruction();

        let ixs = vec![ix; num_reallocations as usize];

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_cast_vote(
        &mut self,
        ncn: Pubkey,
        operator: Pubkey,
        operator_admin: &Keypair,
        meta_merkle_root: [u8; 32],
        epoch: u64,
    ) -> Result<(), TestError> {
        let ncn_config = jito_tip_router_core::ncn_config::NcnConfig::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
        )
        .0;

        let ballot_box = jito_tip_router_core::ballot_box::BallotBox::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
            epoch,
        )
        .0;

        let epoch_snapshot =
            jito_tip_router_core::epoch_snapshot::EpochSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &ncn,
                epoch,
            )
            .0;

        let operator_snapshot =
            jito_tip_router_core::epoch_snapshot::OperatorSnapshot::find_program_address(
                &jito_tip_router_program::id(),
                &operator,
                &ncn,
                epoch,
            )
            .0;

        self.cast_vote(
            ncn_config,
            ballot_box,
            ncn,
            epoch_snapshot,
            operator_snapshot,
            operator,
            operator_admin,
            meta_merkle_root,
            epoch,
        )
        .await
    }

    pub async fn cast_vote(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch_snapshot: Pubkey,
        operator_snapshot: Pubkey,
        operator: Pubkey,
        operator_admin: &Keypair,
        meta_merkle_root: [u8; 32],
        epoch: u64,
    ) -> Result<(), TestError> {
        let ix = CastVoteBuilder::new()
            .ncn_config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .operator(operator)
            .operator_admin(operator_admin.pubkey())
            .restaking_program(jito_restaking_program::id())
            .meta_merkle_root(meta_merkle_root)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, operator_admin],
            blockhash,
        ))
        .await
    }

    pub async fn do_set_merkle_root(
        &mut self,
        ncn: Pubkey,
        vote_account: Pubkey,
        proof: Vec<[u8; 32]>,
        merkle_root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        epoch: u64,
    ) -> Result<(), TestError> {
        let ncn_config = jito_tip_router_core::ncn_config::NcnConfig::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
        )
        .0;
        let ballot_box =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let tip_distribution_program_id = jito_tip_distribution::ID;
        let tip_distribution_account = derive_tip_distribution_account_address(
            &tip_distribution_program_id,
            &vote_account,
            epoch,
        )
        .0;

        let tip_distribution_config =
            jito_tip_distribution_sdk::derive_config_account_address(&tip_distribution_program_id)
                .0;
        let restaking_program_id = jito_restaking_program::id();

        self.set_merkle_root(
            ncn_config,
            ncn,
            ballot_box,
            vote_account,
            tip_distribution_account,
            tip_distribution_config,
            tip_distribution_program_id,
            restaking_program_id,
            proof,
            merkle_root,
            max_total_claim,
            max_num_nodes,
            epoch,
        )
        .await
    }

    pub async fn set_merkle_root(
        &mut self,
        ncn_config: Pubkey,
        ncn: Pubkey,
        ballot_box: Pubkey,
        vote_account: Pubkey,
        tip_distribution_account: Pubkey,
        tip_distribution_config: Pubkey,
        tip_distribution_program_id: Pubkey,
        restaking_program_id: Pubkey,
        proof: Vec<[u8; 32]>,
        merkle_root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        epoch: u64,
    ) -> Result<(), TestError> {
        let ix = SetMerkleRootBuilder::new()
            .ncn_config(ncn_config)
            .ncn(ncn)
            .ballot_box(ballot_box)
            .vote_account(vote_account)
            .tip_distribution_account(tip_distribution_account)
            .tip_distribution_config(tip_distribution_config)
            .tip_distribution_program(tip_distribution_program_id)
            .restaking_program(restaking_program_id)
            .proof(proof)
            .merkle_root(merkle_root)
            .max_total_claim(max_total_claim)
            .max_num_nodes(max_num_nodes)
            .epoch(epoch)
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

    pub async fn do_set_tie_breaker(
        &mut self,
        ncn: Pubkey,
        meta_merkle_root: [u8; 32],
        epoch: u64,
    ) -> Result<(), TestError> {
        let ncn_config = jito_tip_router_core::ncn_config::NcnConfig::find_program_address(
            &jito_tip_router_program::id(),
            &ncn,
        )
        .0;
        let ballot_box =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let tie_breaker_admin = self.payer.pubkey();
        let restaking_program_id = jito_restaking_program::id();

        self.set_tie_breaker(
            ncn_config,
            ballot_box,
            ncn,
            tie_breaker_admin,
            meta_merkle_root,
            epoch,
            restaking_program_id,
        )
        .await
    }

    pub async fn set_tie_breaker(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        tie_breaker_admin: Pubkey,
        meta_merkle_root: [u8; 32],
        epoch: u64,
        restaking_program_id: Pubkey,
    ) -> Result<(), TestError> {
        let ix = SetTieBreakerBuilder::new()
            .ncn_config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .tie_breaker_admin(tie_breaker_admin)
            .meta_merkle_root(meta_merkle_root)
            .epoch(epoch)
            .restaking_program(restaking_program_id)
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

    pub async fn do_full_initialize_base_reward_router(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_initialize_base_reward_router(ncn, epoch).await?;
        let num_reallocs =
            (BaseRewardRouter::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_base_reward_router(ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    pub async fn do_initialize_base_reward_router(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        self.initialize_base_reward_router(restaking_config, ncn, base_reward_router, epoch)
            .await
    }

    pub async fn initialize_base_reward_router(
        &mut self,
        restaking_config: Pubkey,
        ncn: Pubkey,
        base_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = InitializeBaseRewardRouterBuilder::new()
            .restaking_config(restaking_config)
            .ncn(ncn)
            .base_reward_router(base_reward_router)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .epoch(epoch)
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

    pub async fn do_initialize_ncn_reward_router(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        self.initialize_ncn_reward_router(
            ncn_fee_group,
            ncn,
            operator,
            restaking_config,
            ncn_reward_router,
            epoch,
        )
        .await
    }

    pub async fn initialize_ncn_reward_router(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        restaking_config: Pubkey,
        ncn_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = InitializeNcnRewardRouterBuilder::new()
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_reward_router(ncn_reward_router)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
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

    pub async fn do_route_base_rewards(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (epoch_snapshot, _, _) =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (ballot_box, _, _) =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        self.route_base_rewards(
            ncn,
            restaking_config,
            epoch_snapshot,
            ballot_box,
            base_reward_router,
            epoch,
        )
        .await
    }

    pub async fn route_base_rewards(
        &mut self,
        ncn: Pubkey,
        restaking_config: Pubkey,
        epoch_snapshot: Pubkey,
        ballot_box: Pubkey,
        base_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = RouteBaseRewardsBuilder::new()
            .restaking_config(restaking_config)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .ballot_box(ballot_box)
            .base_reward_router(base_reward_router)
            .restaking_program(jito_restaking_program::id())
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                ix,
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_route_ncn_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        );

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        self.route_ncn_rewards(
            ncn_fee_group,
            ncn,
            operator,
            restaking_config,
            operator_snapshot,
            ncn_reward_router,
            epoch,
        )
        .await
    }

    pub async fn route_ncn_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        restaking_config: Pubkey,
        operator_snapshot: Pubkey,
        ncn_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = RouteNcnRewardsBuilder::new()
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .restaking_program(jito_restaking_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[
                // TODO: should make this instruction much more efficient
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                ix,
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_distribute_base_rewards(
        &mut self,
        base_fee_group: BaseFeeGroup,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let ncn_config_account = self.get_ncn_config(ncn).await?;
        let base_fee_wallet = ncn_config_account
            .fee_config
            .base_fee_wallet(base_fee_group)
            .unwrap();

        self.distribute_base_rewards(
            base_fee_group,
            ncn,
            restaking_config,
            ncn_config,
            base_reward_router,
            base_fee_wallet,
            epoch,
        )
        .await
    }

    pub async fn distribute_base_rewards(
        &mut self,
        base_fee_group: BaseFeeGroup,
        ncn: Pubkey,
        restaking_config: Pubkey,
        ncn_config: Pubkey,
        base_reward_router: Pubkey,
        base_fee_wallet: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = DistributeBaseRewardsBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .base_reward_router(base_reward_router)
            .base_fee_wallet(base_fee_wallet)
            .restaking_program(jito_restaking_program::id())
            .base_fee_group(base_fee_group.group)
            .epoch(epoch)
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

    pub async fn do_distribute_base_ncn_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        self.distribute_base_ncn_reward_route(
            ncn_fee_group,
            operator,
            ncn,
            restaking_config,
            ncn_config,
            base_reward_router,
            ncn_reward_router,
            epoch,
        )
        .await
    }

    pub async fn distribute_base_ncn_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        restaking_config: Pubkey,
        ncn_config: Pubkey,
        base_reward_router: Pubkey,
        ncn_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = DistributeBaseNcnRewardRouteBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .base_reward_router(base_reward_router)
            .ncn_reward_router(ncn_reward_router)
            .restaking_program(jito_restaking_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
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

    pub async fn do_distribute_ncn_operator_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        self.distribute_ncn_operator_rewards(
            ncn_fee_group,
            operator,
            ncn,
            restaking_config,
            ncn_config,
            ncn_reward_router,
            epoch,
        )
        .await
    }

    pub async fn distribute_ncn_operator_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        restaking_config: Pubkey,
        ncn_config: Pubkey,
        ncn_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = DistributeNcnOperatorRewardsBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_reward_router(ncn_reward_router)
            .restaking_program(jito_restaking_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
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

    pub async fn do_distribute_ncn_vault_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        self.distribute_ncn_vault_rewards(
            ncn_fee_group,
            vault,
            operator,
            ncn,
            restaking_config,
            ncn_config,
            ncn_reward_router,
            epoch,
        )
        .await
    }

    pub async fn distribute_ncn_vault_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        vault: Pubkey,
        operator: Pubkey,
        ncn: Pubkey,
        restaking_config: Pubkey,
        ncn_config: Pubkey,
        ncn_reward_router: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let ix = DistributeNcnVaultRewardsBuilder::new()
            .restaking_config(restaking_config)
            .ncn_config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .vault(vault)
            .ncn_reward_router(ncn_reward_router)
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
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
    pub async fn do_realloc_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let restaking_config = Config::find_program_address(&jito_restaking_program::id()).0;
        let ncn_operator_state =
            NcnOperatorState::find_program_address(&jito_restaking_program::id(), &ncn, &operator)
                .0;
        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        )
        .0;

        self.realloc_operator_snapshot(
            ncn_config,
            restaking_config,
            ncn,
            operator,
            ncn_operator_state,
            epoch_snapshot,
            operator_snapshot,
            epoch,
            num_reallocations,
        )
        .await
    }

    pub async fn realloc_operator_snapshot(
        &mut self,
        ncn_config: Pubkey,
        restaking_config: Pubkey,
        ncn: Pubkey,
        operator: Pubkey,
        ncn_operator_state: Pubkey,
        epoch_snapshot: Pubkey,
        operator_snapshot: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ix = ReallocOperatorSnapshotBuilder::new()
            .ncn_config(ncn_config)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .payer(self.payer.pubkey())
            .restaking_program(jito_restaking_program::id())
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        let ixs = vec![ix; num_reallocations as usize];

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_realloc_base_reward_router(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let base_reward_router =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        self.realloc_base_reward_router(
            ncn_config,
            base_reward_router,
            ncn,
            epoch,
            num_reallocations,
        )
        .await
    }

    pub async fn realloc_base_reward_router(
        &mut self,
        ncn_config: Pubkey,
        base_reward_router: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ix = ReallocBaseRewardRouterBuilder::new()
            .ncn_config(ncn_config)
            .base_reward_router(base_reward_router)
            .ncn(ncn)
            .epoch(epoch)
            .payer(self.payer.pubkey())
            .system_program(system_program::id())
            .instruction();

        let ixs = vec![ix; num_reallocations as usize];

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_realloc_weight_table(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let tracked_mints =
            TrackedMints::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        self.realloc_weight_table(
            ncn_config,
            weight_table,
            ncn,
            tracked_mints,
            epoch,
            num_reallocations,
        )
        .await
    }

    pub async fn realloc_weight_table(
        &mut self,
        ncn_config: Pubkey,
        weight_table: Pubkey,
        ncn: Pubkey,
        tracked_mints: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let ix = ReallocWeightTableBuilder::new()
            .ncn_config(ncn_config)
            .weight_table(weight_table)
            .ncn(ncn)
            .tracked_mints(tracked_mints)
            .epoch(epoch)
            .payer(self.payer.pubkey())
            .system_program(system_program::id())
            .instruction();

        let ixs = vec![ix; num_reallocations as usize];

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &ixs,
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

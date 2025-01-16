use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn_operator_state::NcnOperatorState, ncn_vault_ticket::NcnVaultTicket,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, jito_tip_distribution};
use jito_tip_router_client::{
    instructions::{
        AdminRegisterStMintBuilder, AdminSetConfigFeesBuilder, AdminSetNewAdminBuilder,
        AdminSetParametersBuilder, AdminSetStMintBuilder, AdminSetTieBreakerBuilder,
        AdminSetWeightBuilder, CastVoteBuilder, ClaimWithPayerBuilder,
        DistributeBaseNcnRewardRouteBuilder, DistributeBaseRewardsBuilder,
        DistributeNcnOperatorRewardsBuilder, DistributeNcnVaultRewardsBuilder,
        InitializeBallotBoxBuilder, InitializeBaseRewardRouterBuilder, InitializeConfigBuilder,
        InitializeEpochSnapshotBuilder, InitializeEpochStateBuilder,
        InitializeNcnRewardRouterBuilder, InitializeOperatorSnapshotBuilder,
        InitializeVaultRegistryBuilder, InitializeWeightTableBuilder, ReallocBallotBoxBuilder,
        ReallocBaseRewardRouterBuilder, ReallocEpochStateBuilder, ReallocOperatorSnapshotBuilder,
        ReallocVaultRegistryBuilder, ReallocWeightTableBuilder, RegisterVaultBuilder,
        RouteBaseRewardsBuilder, RouteNcnRewardsBuilder, SetMerkleRootBuilder,
        SnapshotVaultOperatorDelegationBuilder, SwitchboardSetWeightBuilder,
    },
    types::ConfigAdminRole,
};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_fee_group::BaseFeeGroup,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    claim_status_payer::ClaimStatusPayer,
    config::Config as NcnConfig,
    constants::{JITOSOL_MINT, MAX_REALLOC_BYTES},
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
use jito_vault_core::{
    vault_ncn_ticket::VaultNcnTicket, vault_operator_delegation::VaultOperatorDelegation,
};
use solana_program::{
    hash::Hash, instruction::InstructionError, native_token::sol_to_lamports, pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_program_test::{BanksClient, ProgramTestBanksClientExt};
use solana_sdk::{
    commitment_config::CommitmentLevel,
    compute_budget::ComputeBudgetInstruction,
    signature::{Keypair, Signer},
    system_program,
    transaction::{Transaction, TransactionError},
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account_idempotent,
};
use spl_stake_pool::find_withdraw_authority_program_address;

use super::{restaking_client::NcnRoot, stake_pool_client::PoolRoot};
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

    pub async fn get_best_latest_blockhash(&mut self) -> TestResult<Hash> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        let new_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&blockhash)
            .await?;

        Ok(new_blockhash)
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

    pub async fn airdrop_lamports(&mut self, to: &Pubkey, lamports: u64) -> TestResult<()> {
        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(&self.payer.pubkey(), to, lamports)],
                    Some(&self.payer.pubkey()),
                    &[&self.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await?;
        Ok(())
    }

    pub async fn setup_tip_router(&mut self, ncn_root: &NcnRoot) -> TestResult<()> {
        self.do_initialize_config(ncn_root.ncn_pubkey, &ncn_root.ncn_admin)
            .await?;
        self.do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
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

    pub async fn get_vault_registry(&mut self, ncn_pubkey: Pubkey) -> TestResult<VaultRegistry> {
        let vault_registry_pda =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn_pubkey).0;
        let vault_registry = self
            .banks_client
            .get_account(vault_registry_pda)
            .await?
            .unwrap();
        Ok(*VaultRegistry::try_from_slice_unchecked(vault_registry.data.as_slice()).unwrap())
    }

    pub async fn get_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<EpochState> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let raw_account = self.banks_client.get_account(epoch_state).await?.unwrap();
        Ok(*EpochState::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
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
            3,
            10000,
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
        epochs_before_stall: u64,
        valid_slots_after_consensus: u64,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = InitializeConfigBuilder::new()
            .config(ncn_config)
            .ncn(ncn)
            .ncn_admin(ncn_admin.pubkey())
            .fee_wallet(*fee_wallet)
            .tie_breaker_admin(*tie_breaker_admin)
            .dao_fee_bps(dao_fee_bps)
            .default_ncn_fee_bps(default_ncn_fee_bps)
            .block_engine_fee_bps(block_engine_fee_bps)
            .epochs_before_stall(epochs_before_stall)
            .valid_slots_after_consensus(valid_slots_after_consensus)
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
        let ix = {
            let mut builder = AdminSetConfigFeesBuilder::new();
            builder
                .config(config_pda)
                .ncn(ncn_root.ncn_pubkey)
                .ncn_admin(ncn_root.ncn_admin.pubkey());

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
        let ix = AdminSetNewAdminBuilder::new()
            .config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey())
            .new_admin(new_admin)
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

    pub async fn do_full_initialize_epoch_state(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        self.do_intialize_epoch_state(ncn, epoch).await?;
        let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_epoch_state(ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    pub async fn do_intialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.initialize_epoch_state(ncn, epoch).await
    }

    pub async fn initialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = InitializeEpochStateBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .payer(self.payer.pubkey())
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

    pub async fn do_realloc_epoch_state(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> TestResult<()> {
        self.realloc_epoch_state(ncn, epoch, num_reallocations)
            .await
    }

    pub async fn realloc_epoch_state(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = ReallocEpochStateBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .payer(self.payer.pubkey())
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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = InitializeWeightTableBuilder::new()
            .epoch_state(epoch_state)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .weight_table(weight_table)
            .payer(self.payer.pubkey())
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

    pub async fn do_admin_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        self.admin_set_weight(ncn, epoch, st_mint, weight).await
    }

    pub async fn admin_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
        weight: u128,
    ) -> TestResult<()> {
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = AdminSetWeightBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .weight_table(weight_table)
            .weight_table_admin(self.payer.pubkey())
            .st_mint(st_mint)
            .weight(weight)
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

    pub async fn do_switchboard_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
    ) -> TestResult<()> {
        let vault_registry = self.get_vault_registry(ncn).await?;

        let mint_entry = vault_registry.get_mint_entry(&st_mint)?;
        let switchboard_feed = mint_entry.switchboard_feed();

        self.switchboard_set_weight(ncn, epoch, st_mint, *switchboard_feed)
            .await
    }

    pub async fn switchboard_set_weight(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        st_mint: Pubkey,
        switchboard_feed: Pubkey,
    ) -> TestResult<()> {
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = SwitchboardSetWeightBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .weight_table(weight_table)
            .st_mint(st_mint)
            .switchboard_feed(switchboard_feed)
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

    pub async fn do_full_initialize_vault_registry(&mut self, ncn: Pubkey) -> TestResult<()> {
        self.do_initialize_vault_registry(ncn).await?;
        let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_vault_registry(ncn, num_reallocs).await?;
        Ok(())
    }

    pub async fn do_initialize_vault_registry(&mut self, ncn: Pubkey) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        self.initialize_vault_registry(&ncn_config, &vault_registry, &ncn)
            .await
    }

    pub async fn initialize_vault_registry(
        &mut self,
        ncn_config: &Pubkey,
        vault_registry: &Pubkey,
        ncn: &Pubkey,
    ) -> TestResult<()> {
        let ix = InitializeVaultRegistryBuilder::new()
            .config(*ncn_config)
            .vault_registry(*vault_registry)
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

    pub async fn do_realloc_vault_registry(
        &mut self,
        ncn: Pubkey,
        num_reallocations: u64,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        self.realloc_vault_registry(&ncn, &ncn_config, &vault_registry, num_reallocations)
            .await
    }

    pub async fn realloc_vault_registry(
        &mut self,
        ncn: &Pubkey,
        config: &Pubkey,
        vault_registry: &Pubkey,
        num_reallocations: u64,
    ) -> TestResult<()> {
        let ix = ReallocVaultRegistryBuilder::new()
            .ncn(*ncn)
            .payer(self.payer.pubkey())
            .config(*config)
            .vault_registry(*vault_registry)
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

    pub async fn do_register_vault(
        &mut self,
        ncn: Pubkey,
        vault: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        self.register_vault(ncn_config, vault_registry, ncn, vault, ncn_vault_ticket)
            .await
    }

    pub async fn register_vault(
        &mut self,
        config: Pubkey,
        vault_registry: Pubkey,
        ncn: Pubkey,
        vault: Pubkey,
        ncn_vault_ticket: Pubkey,
    ) -> TestResult<()> {
        let ix = RegisterVaultBuilder::new()
            .config(config)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .vault(vault)
            .ncn_vault_ticket(ncn_vault_ticket)
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

    pub async fn do_admin_register_st_mint(
        &mut self,
        ncn: Pubkey,
        st_mint: Pubkey,
        ncn_fee_group: NcnFeeGroup,
        reward_multiplier_bps: u64,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    ) -> TestResult<()> {
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let admin = self.payer.pubkey();

        self.admin_register_st_mint(
            ncn,
            ncn_config,
            vault_registry,
            admin,
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        )
        .await
    }

    pub async fn admin_register_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
        ncn_fee_group: NcnFeeGroup,
        reward_multiplier_bps: u64,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    ) -> TestResult<()> {
        let ix = {
            let mut builder = AdminRegisterStMintBuilder::new();
            builder
                .config(ncn_config)
                .ncn(ncn)
                .vault_registry(vault_registry)
                .admin(admin)
                .st_mint(st_mint)
                .ncn_fee_group(ncn_fee_group.group)
                .reward_multiplier_bps(reward_multiplier_bps);

            if let Some(switchboard_feed) = switchboard_feed {
                builder.switchboard_feed(switchboard_feed);
            }

            if let Some(no_feed_weight) = no_feed_weight {
                builder.no_feed_weight(no_feed_weight);
            }

            builder.instruction()
        };

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        ))
        .await
    }

    pub async fn do_admin_set_st_mint(
        &mut self,
        ncn: Pubkey,
        st_mint: Pubkey,
        ncn_fee_group: Option<NcnFeeGroup>,
        reward_multiplier_bps: Option<u64>,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    ) -> TestResult<()> {
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let admin = self.payer.pubkey();

        self.admin_set_st_mint(
            ncn,
            ncn_config,
            vault_registry,
            admin,
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        )
        .await
    }

    pub async fn admin_set_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
        ncn_fee_group: Option<NcnFeeGroup>,
        reward_multiplier_bps: Option<u64>,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    ) -> TestResult<()> {
        let ix = {
            let mut builder = AdminSetStMintBuilder::new();
            builder
                .config(ncn_config)
                .ncn(ncn)
                .vault_registry(vault_registry)
                .admin(admin)
                .st_mint(st_mint);

            if let Some(ncn_fee_group) = ncn_fee_group {
                builder.ncn_fee_group(ncn_fee_group.group);
            }

            if let Some(reward_multiplier_bps) = reward_multiplier_bps {
                builder.reward_multiplier_bps(reward_multiplier_bps);
            }

            if let Some(switchboard_feed) = switchboard_feed {
                builder.switchboard_feed(switchboard_feed);
            }

            if let Some(no_feed_weight) = no_feed_weight {
                builder.no_feed_weight(no_feed_weight);
            }

            builder.instruction()
        };

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
        let config_pda = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = InitializeEpochSnapshotBuilder::new()
            .epoch_state(epoch_state)
            .config(config_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .payer(self.payer.pubkey())
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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
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
            .epoch_state(epoch_state)
            .config(config_pda)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .payer(self.payer.pubkey())
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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
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

        let ix = SnapshotVaultOperatorDelegationBuilder::new()
            .epoch_state(epoch_state)
            .config(config_pda)
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
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

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
        config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> Result<(), TestError> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = InitializeBallotBoxBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
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
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

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
        config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = ReallocBallotBoxBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
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
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = CastVoteBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .operator(operator)
            .operator_admin(operator_admin.pubkey())
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
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
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

        self.set_merkle_root(
            ncn_config,
            ncn,
            ballot_box,
            vote_account,
            tip_distribution_account,
            tip_distribution_config,
            tip_distribution_program_id,
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
        proof: Vec<[u8; 32]>,
        merkle_root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        epoch: u64,
    ) -> Result<(), TestError> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = SetMerkleRootBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ncn(ncn)
            .ballot_box(ballot_box)
            .vote_account(vote_account)
            .tip_distribution_account(tip_distribution_account)
            .tip_distribution_config(tip_distribution_config)
            .tip_distribution_program(tip_distribution_program_id)
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

    pub async fn do_admin_set_tie_breaker(
        &mut self,
        ncn: Pubkey,
        meta_merkle_root: [u8; 32],
        epoch: u64,
    ) -> Result<(), TestError> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let ballot_box =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let tie_breaker_admin = self.payer.pubkey();

        self.admin_set_tie_breaker(
            ncn_config,
            ballot_box,
            ncn,
            tie_breaker_admin,
            meta_merkle_root,
            epoch,
        )
        .await
    }

    pub async fn admin_set_tie_breaker(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        tie_breaker_admin: Pubkey,
        meta_merkle_root: [u8; 32],
        epoch: u64,
    ) -> Result<(), TestError> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = AdminSetTieBreakerBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .tie_breaker_admin(tie_breaker_admin)
            .meta_merkle_root(meta_merkle_root)
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
        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        self.initialize_base_reward_router(ncn, base_reward_router, base_reward_receiver, epoch)
            .await
    }

    pub async fn initialize_base_reward_router(
        &mut self,
        ncn: Pubkey,
        base_reward_router: Pubkey,
        base_reward_receiver: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = InitializeBaseRewardRouterBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .payer(self.payer.pubkey())
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
        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        );

        self.initialize_ncn_reward_router(
            ncn_fee_group,
            ncn,
            operator,
            operator_snapshot,
            ncn_reward_router,
            ncn_reward_receiver,
            epoch,
        )
        .await
    }

    pub async fn initialize_ncn_reward_router(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        operator_snapshot: Pubkey,
        ncn_reward_router: Pubkey,
        ncn_reward_receiver: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let ix = InitializeNcnRewardRouterBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .operator(operator)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .payer(self.payer.pubkey())
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
        let (epoch_snapshot, _, _) =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (ballot_box, _, _) =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        //Pretty close to max
        let max_iterations: u16 = BaseRewardRouter::MAX_ROUTE_BASE_ITERATIONS;

        let mut still_routing = true;
        while still_routing {
            self.route_base_rewards(
                ncn,
                epoch_snapshot,
                ballot_box,
                base_reward_router,
                base_reward_receiver,
                max_iterations,
                epoch,
            )
            .await?;

            let base_reward_router_account = self.get_base_reward_router(ncn, epoch).await?;

            still_routing = base_reward_router_account.still_routing();
        }

        Ok(())
    }

    pub async fn route_base_rewards(
        &mut self,
        ncn: Pubkey,
        epoch_snapshot: Pubkey,
        ballot_box: Pubkey,
        base_reward_router: Pubkey,
        base_reward_receiver: Pubkey,
        max_iterations: u16,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let ix = RouteBaseRewardsBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .ballot_box(ballot_box)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .max_iterations(max_iterations)
            .epoch(epoch)
            .instruction();

        let blockhash = self.get_best_latest_blockhash().await?;
        let tx = &Transaction::new_signed_with_payer(
            &[
                ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
                ix,
            ],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );

        {
            let simulation = self
                .banks_client
                .simulate_transaction(tx.clone())
                .await
                .unwrap();

            let details = simulation.simulation_details.unwrap();

            println!("\n -------- TX ----------");
            println!("CU: {:?}/{}\n", details.units_consumed, 1_400_000);
            println!("{:?}\n", details);
            println!("\n ----------------------");
        }

        self.process_transaction(tx).await
    }

    pub async fn do_route_ncn_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
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

        let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        let max_iterations: u16 = NcnRewardRouter::MAX_ROUTE_NCN_ITERATIONS;
        let mut still_routing = true;

        while still_routing {
            self.route_ncn_rewards(
                ncn_fee_group,
                ncn,
                operator,
                operator_snapshot,
                ncn_reward_router,
                ncn_reward_receiver,
                max_iterations,
                epoch,
            )
            .await?;

            let ncn_reward_router_account = self
                .get_ncn_reward_router(ncn_fee_group, operator, ncn, epoch)
                .await?;

            still_routing = ncn_reward_router_account.still_routing();
        }

        Ok(())
    }

    pub async fn route_ncn_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        ncn: Pubkey,
        operator: Pubkey,
        operator_snapshot: Pubkey,
        ncn_reward_router: Pubkey,
        ncn_reward_receiver: Pubkey,
        max_iterations: u16,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = RouteNcnRewardsBuilder::new()
            .epoch_state(epoch_state)
            .ncn(ncn)
            .operator(operator)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .ncn_fee_group(ncn_fee_group.group)
            .max_iterations(max_iterations)
            .epoch(epoch)
            .instruction();

        let blockhash = self.get_best_latest_blockhash().await?;
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
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let ncn_config_account = self.get_ncn_config(ncn).await?;
        let base_fee_wallet = ncn_config_account
            .fee_config
            .base_fee_wallet(base_fee_group)
            .unwrap();
        let base_fee_wallet_ata = get_associated_token_address(&base_fee_wallet, &JITOSOL_MINT);
        let create_base_fee_wallet_ata_ix = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &base_fee_wallet,
            &JITOSOL_MINT,
            &spl_token::id(),
        );
        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        // stake pool accounts
        let stake_pool = pool_root.pool_address;
        let (stake_pool_withdraw_authority, _) =
            find_withdraw_authority_program_address(&spl_stake_pool::id(), &stake_pool);
        let reserve_stake = pool_root.reserve_stake;
        let manager_fee_account = pool_root.manager_fee_account;
        let referrer_pool_tokens_account = pool_root.referrer_pool_tokens_account;

        let ix = DistributeBaseRewardsBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ncn(ncn)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .base_fee_wallet(*base_fee_wallet)
            .base_fee_wallet_ata(base_fee_wallet_ata)
            .stake_pool_program(spl_stake_pool::id())
            .stake_pool(stake_pool)
            .stake_pool_withdraw_authority(stake_pool_withdraw_authority)
            .reserve_stake(reserve_stake)
            .manager_fee_account(manager_fee_account)
            .referrer_pool_tokens_account(referrer_pool_tokens_account)
            .pool_mint(JITOSOL_MINT)
            .token_program(spl_token::id())
            .system_program(system_program::id())
            .base_fee_group(base_fee_group.group)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;

        let transaction = Transaction::new_signed_with_payer(
            &[create_base_fee_wallet_ata_ix, ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );

        self.process_transaction(&transaction).await
    }

    pub async fn do_distribute_base_ncn_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let (ncn_config, _, _) =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (base_reward_router, _, _) =
            BaseRewardRouter::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
        let (base_reward_receiver, _, _) =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );
        let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
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
            ncn_config,
            base_reward_router,
            base_reward_receiver,
            ncn_reward_router,
            ncn_reward_receiver,
            epoch,
        )
        .await
    }

    pub async fn distribute_base_ncn_reward_route(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        ncn_config: Pubkey,
        base_reward_router: Pubkey,
        base_reward_receiver: Pubkey,
        ncn_reward_router: Pubkey,
        ncn_reward_receiver: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = DistributeBaseNcnRewardRouteBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
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

    pub async fn do_distribute_ncn_operator_rewards(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        // Add stake pool accounts
        let stake_pool = pool_root.pool_address;
        let (stake_pool_withdraw_authority, _) =
            find_withdraw_authority_program_address(&spl_stake_pool::id(), &stake_pool);
        let reserve_stake = pool_root.reserve_stake;
        let manager_fee_account = pool_root.manager_fee_account;
        let referrer_pool_tokens_account = pool_root.referrer_pool_tokens_account;

        let operator_ata = get_associated_token_address(&operator, &JITOSOL_MINT);
        let operator_ata_ix = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &operator,
            &JITOSOL_MINT,
            &spl_token::id(),
        );

        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        )
        .0;

        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = DistributeNcnOperatorRewardsBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .operator_ata(operator_ata)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .stake_pool_program(spl_stake_pool::id())
            .stake_pool(stake_pool)
            .stake_pool_withdraw_authority(stake_pool_withdraw_authority)
            .reserve_stake(reserve_stake)
            .manager_fee_account(manager_fee_account)
            .referrer_pool_tokens_account(referrer_pool_tokens_account)
            .pool_mint(JITOSOL_MINT)
            .token_program(spl_token::id())
            .system_program(system_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[operator_ata_ix, ix],
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
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let ncn_config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );
        let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
            &jito_tip_router_program::id(),
            ncn_fee_group,
            &operator,
            &ncn,
            epoch,
        );

        let operator_snapshot = OperatorSnapshot::find_program_address(
            &jito_tip_router_program::id(),
            &operator,
            &ncn,
            epoch,
        )
        .0;

        // Add stake pool accounts
        let stake_pool = pool_root.pool_address;
        let (stake_pool_withdraw_authority, _) =
            find_withdraw_authority_program_address(&spl_stake_pool::id(), &stake_pool);
        let reserve_stake = pool_root.reserve_stake;
        let manager_fee_account = pool_root.manager_fee_account;
        let referrer_pool_tokens_account = pool_root.referrer_pool_tokens_account;

        let vault_ata = get_associated_token_address(&vault, &JITOSOL_MINT);

        let vault_ata_ix = create_associated_token_account_idempotent(
            &self.payer.pubkey(),
            &vault,
            &JITOSOL_MINT,
            &spl_token::id(),
        );
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = DistributeNcnVaultRewardsBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .ncn(ncn)
            .operator(operator)
            .vault(vault)
            .vault_ata(vault_ata)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .stake_pool_program(spl_stake_pool::id())
            .stake_pool(stake_pool)
            .stake_pool_withdraw_authority(stake_pool_withdraw_authority)
            .reserve_stake(reserve_stake)
            .manager_fee_account(manager_fee_account)
            .referrer_pool_tokens_account(referrer_pool_tokens_account)
            .pool_mint(JITOSOL_MINT)
            .token_program(spl_token::id())
            .system_program(system_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[vault_ata_ix, ix],
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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = ReallocOperatorSnapshotBuilder::new()
            .epoch_state(epoch_state)
            .ncn_config(ncn_config)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .payer(self.payer.pubkey())
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
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = ReallocBaseRewardRouterBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
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
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        self.realloc_weight_table(
            ncn_config,
            weight_table,
            ncn,
            vault_registry,
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
        vault_registry: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let ix = ReallocWeightTableBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .weight_table(weight_table)
            .ncn(ncn)
            .vault_registry(vault_registry)
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

    pub async fn do_claim_with_payer(
        &mut self,
        claimant: Pubkey,
        tip_distribution_account: Pubkey,
        proof: Vec<[u8; 32]>,
        amount: u64,
    ) -> TestResult<()> {
        let (claim_status_payer, _, _) = ClaimStatusPayer::find_program_address(
            &jito_tip_router_program::id(),
            &jito_tip_distribution::ID,
        );

        let tip_distribution_program_id = jito_tip_distribution::ID;
        let tip_distribution_config =
            jito_tip_distribution_sdk::derive_config_account_address(&tip_distribution_program_id)
                .0;

        let (claim_status, claim_status_bump) =
            jito_tip_distribution_sdk::derive_claim_status_account_address(
                &tip_distribution_program_id,
                &claimant,
                &tip_distribution_account,
            );

        self.claim_with_payer(
            claim_status_payer,
            tip_distribution_program_id,
            tip_distribution_config,
            tip_distribution_account,
            claim_status,
            claimant,
            proof,
            amount,
            claim_status_bump,
        )
        .await
    }

    pub async fn claim_with_payer(
        &mut self,
        claim_status_payer: Pubkey,
        tip_distribution_program: Pubkey,
        tip_distribution_config: Pubkey,
        tip_distribution_account: Pubkey,
        claim_status: Pubkey,
        claimant: Pubkey,
        proof: Vec<[u8; 32]>,
        amount: u64,
        bump: u8,
    ) -> TestResult<()> {
        let ix = ClaimWithPayerBuilder::new()
            .claim_status_payer(claim_status_payer)
            .tip_distribution_program(tip_distribution_program)
            .config(tip_distribution_config)
            .tip_distribution_account(tip_distribution_account)
            .claim_status(claim_status)
            .claimant(claimant)
            .system_program(system_program::id())
            .proof(proof)
            .amount(amount)
            .bump(bump)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            blockhash,
        );

        self.process_transaction(&tx).await
    }

    pub async fn do_set_parameters(
        &mut self,
        epochs_before_stall: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_root.ncn_pubkey).0;

        let mut ix = AdminSetParametersBuilder::new();
        ix.config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey());

        if let Some(epochs) = epochs_before_stall {
            ix.epochs_before_stall(epochs);
        }

        if let Some(slots) = valid_slots_after_consensus {
            ix.valid_slots_after_consensus(slots);
        }

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix.instruction()],
            Some(&ncn_root.ncn_admin.pubkey()),
            &[&ncn_root.ncn_admin],
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

use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config, ncn_operator_state::NcnOperatorState, ncn_vault_ticket::NcnVaultTicket,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, jito_tip_distribution};
use jito_tip_router_client::{
    instructions::{
        AdminRegisterStMintBuilder, AdminSetNewAdminBuilder, AdminSetParametersBuilder,
        AdminSetStMintBuilder, AdminSetTieBreakerBuilder, AdminSetWeightBuilder, CastVoteBuilder,
        CloseEpochAccountBuilder, InitializeBallotBoxBuilder, InitializeConfigBuilder,
        InitializeEpochSnapshotBuilder, InitializeEpochStateBuilder,
        InitializeOperatorSnapshotBuilder, InitializeVaultRegistryBuilder,
        InitializeWeightTableBuilder, ReallocBallotBoxBuilder, ReallocEpochStateBuilder,
        ReallocOperatorSnapshotBuilder, ReallocVaultRegistryBuilder, ReallocWeightTableBuilder,
        RegisterVaultBuilder, SetMerkleRootBuilder, SnapshotVaultOperatorDelegationBuilder,
        SwitchboardSetWeightBuilder,
    },
    types::ConfigAdminRole,
};
use jito_tip_router_core::{
    account_payer::AccountPayer,
    ballot_box::BallotBox,
    config::Config as NcnConfig,
    constants::MAX_REALLOC_BYTES,
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    error::TipRouterError,
    vault_registry::VaultRegistry,
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

        self.do_full_initialize_vault_registry(ncn_root.ncn_pubkey)
            .await?;

        Ok(())
    }

    pub async fn get_epoch_marker(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<EpochMarker> {
        let epoch_marker =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let raw_account = self.banks_client.get_account(epoch_marker).await?.unwrap();
        Ok(*EpochMarker::try_from_slice_unchecked(raw_account.data.as_slice()).unwrap())
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

    pub async fn do_initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
    ) -> TestResult<()> {
        // Setup Payer
        self.airdrop(&self.payer.pubkey(), 1.0).await?;

        // Setup account payer
        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);
        self.airdrop(&account_payer, 100.0).await?;

        let ncn_admin_pubkey = ncn_admin.pubkey();
        self.initialize_config(ncn, ncn_admin, &ncn_admin_pubkey, 3, 10, 10000)
            .await
    }

    pub async fn initialize_config(
        &mut self,
        ncn: Pubkey,
        ncn_admin: &Keypair,
        tie_breaker_admin: &Pubkey,
        epochs_before_stall: u64,
        epochs_after_consensus_before_close: u64,
        valid_slots_after_consensus: u64,
    ) -> TestResult<()> {
        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeConfigBuilder::new()
            .config(config)
            .ncn(ncn)
            .ncn_admin(ncn_admin.pubkey())
            .account_payer(account_payer)
            .tie_breaker_admin(*tie_breaker_admin)
            .epochs_before_stall(epochs_before_stall)
            .epochs_after_consensus_before_close(epochs_after_consensus_before_close)
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
        let num_reallocs = (EpochState::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;
        self.do_realloc_epoch_state(ncn, epoch, num_reallocs)
            .await?;
        Ok(())
    }

    pub async fn do_intialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        self.initialize_epoch_state(ncn, epoch).await
    }

    pub async fn initialize_epoch_state(&mut self, ncn: Pubkey, epoch: u64) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeEpochStateBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .account_payer(account_payer)
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

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = ReallocEpochStateBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .ncn(ncn)
            .account_payer(account_payer)
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
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let vault_registry =
            VaultRegistry::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeWeightTableBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .vault_registry(vault_registry)
            .ncn(ncn)
            .weight_table(weight_table)
            .account_payer(account_payer)
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
        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), ncn);

        let ix = InitializeVaultRegistryBuilder::new()
            .config(*ncn_config)
            .vault_registry(*vault_registry)
            .ncn(*ncn)
            .account_payer(account_payer)
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
        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), ncn);

        let ix = ReallocVaultRegistryBuilder::new()
            .ncn(*ncn)
            .account_payer(account_payer)
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
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn admin_register_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
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
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn admin_set_st_mint(
        &mut self,
        ncn: Pubkey,
        ncn_config: Pubkey,
        vault_registry: Pubkey,
        admin: Pubkey,
        st_mint: Pubkey,
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
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let weight_table =
            WeightTable::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;
        let epoch_snapshot =
            EpochSnapshot::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeEpochSnapshotBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .config(config_pda)
            .ncn(ncn)
            .weight_table(weight_table)
            .epoch_snapshot(epoch_snapshot)
            .account_payer(account_payer)
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
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
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

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeOperatorSnapshotBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .config(config_pda)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .account_payer(account_payer)
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
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);
        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = InitializeBallotBoxBuilder::new()
            .epoch_marker(epoch_marker)
            .epoch_state(epoch_state)
            .config(config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .account_payer(account_payer)
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

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = ReallocBallotBoxBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .account_payer(account_payer)
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

    #[allow(clippy::too_many_arguments)]
    pub async fn cast_vote(
        &mut self,
        ncn_config: Pubkey,
        ballot_box: Pubkey,
        ncn: Pubkey,
        epoch_snapshot: Pubkey,
        operator_snapshot: Pubkey,
        operator: Pubkey,
        operator_voter: &Keypair,
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
            .operator_voter(operator_voter.pubkey())
            .meta_merkle_root(meta_merkle_root)
            .epoch(epoch)
            .instruction();

        let blockhash = self.banks_client.get_latest_blockhash().await?;
        self.process_transaction(&Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer, operator_voter],
            blockhash,
        ))
        .await
    }

    #[allow(clippy::too_many_arguments)]
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
        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
        let ballot_box =
            BallotBox::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let tip_distribution_program = jito_tip_distribution::ID;
        let tip_distribution_account = derive_tip_distribution_account_address(
            &tip_distribution_program,
            &vote_account,
            epoch - 1,
        )
        .0;

        let tip_distribution_config =
            jito_tip_distribution_sdk::derive_config_account_address(&tip_distribution_program).0;

        self.set_merkle_root(
            config,
            ncn,
            ballot_box,
            vote_account,
            tip_distribution_account,
            tip_distribution_config,
            tip_distribution_program,
            proof,
            merkle_root,
            max_total_claim,
            max_num_nodes,
            epoch,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn set_merkle_root(
        &mut self,
        config: Pubkey,
        ncn: Pubkey,
        ballot_box: Pubkey,
        vote_account: Pubkey,
        tip_distribution_account: Pubkey,
        tip_distribution_config: Pubkey,
        tip_distribution_program: Pubkey,
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
            .config(config)
            .ncn(ncn)
            .ballot_box(ballot_box)
            .vote_account(vote_account)
            .tip_distribution_account(tip_distribution_account)
            .tip_distribution_config(tip_distribution_config)
            .tip_distribution_program(tip_distribution_program)
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

    pub async fn do_realloc_operator_snapshot(
        &mut self,
        operator: Pubkey,
        ncn: Pubkey,
        epoch: u64,
        num_reallocations: u64,
    ) -> Result<(), TestError> {
        let config = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn).0;
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
            config,
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

    #[allow(clippy::too_many_arguments)]
    pub async fn realloc_operator_snapshot(
        &mut self,
        config: Pubkey,
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

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = ReallocOperatorSnapshotBuilder::new()
            .epoch_state(epoch_state)
            .config(config)
            .restaking_config(restaking_config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .account_payer(account_payer)
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

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let ix = ReallocWeightTableBuilder::new()
            .epoch_state(epoch_state)
            .config(ncn_config)
            .weight_table(weight_table)
            .ncn(ncn)
            .vault_registry(vault_registry)
            .epoch(epoch)
            .account_payer(account_payer)
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

    pub async fn do_close_epoch_account(
        &mut self,
        ncn: Pubkey,
        epoch: u64,
        account_to_close: Pubkey,
    ) -> TestResult<()> {
        let (epoch_marker, _, _) =
            EpochMarker::find_program_address(&jito_tip_router_program::id(), &ncn, epoch);

        let epoch_state =
            EpochState::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let (account_payer, _, _) =
            AccountPayer::find_program_address(&jito_tip_router_program::id(), &ncn);

        let (config, _, _) = NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn);

        self.close_epoch_account(
            epoch_marker,
            epoch_state,
            ncn,
            config,
            account_to_close,
            account_payer,
            epoch,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn close_epoch_account(
        &mut self,
        epoch_marker: Pubkey,
        epoch_state: Pubkey,
        ncn: Pubkey,
        config: Pubkey,
        account_to_close: Pubkey,
        account_payer: Pubkey,
        epoch: u64,
    ) -> TestResult<()> {
        let mut ix = CloseEpochAccountBuilder::new();

        ix.account_payer(account_payer)
            .epoch_marker(epoch_marker)
            .config(config)
            .account_to_close(account_to_close)
            .epoch_state(epoch_state)
            .ncn(ncn)
            .system_program(system_program::id())
            .epoch(epoch);

        let ix = ix.instruction();

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
        starting_valid_epoch: Option<u64>,
        epochs_before_stall: Option<u64>,
        epochs_after_consensus_before_close: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
        ncn_root: &NcnRoot,
    ) -> TestResult<()> {
        let config_pda =
            NcnConfig::find_program_address(&jito_tip_router_program::id(), &ncn_root.ncn_pubkey).0;

        let mut ix = AdminSetParametersBuilder::new();
        ix.config(config_pda)
            .ncn(ncn_root.ncn_pubkey)
            .ncn_admin(ncn_root.ncn_admin.pubkey());

        if let Some(epoch) = starting_valid_epoch {
            ix.starting_valid_epoch(epoch);
        }

        if let Some(epochs) = epochs_before_stall {
            ix.epochs_before_stall(epochs);
        }

        if let Some(epochs) = epochs_after_consensus_before_close {
            ix.epochs_after_consensus_before_close(epochs);
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

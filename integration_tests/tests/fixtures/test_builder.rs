use std::{
    borrow::BorrowMut,
    fmt::{Debug, Formatter},
};

use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
use jito_tip_distribution_sdk::jito_tip_distribution;
use jito_tip_router_core::{
    base_fee_group::BaseFeeGroup,
    base_reward_router::BaseRewardReceiver,
    constants::{JITOSOL_MINT, JTO_SOL_FEED},
    ncn_fee_group::NcnFeeGroup,
};
use jito_vault_core::vault_ncn_ticket::VaultNcnTicket;
use solana_program::{
    clock::Clock, native_token::sol_to_lamports, program_pack::Pack, pubkey::Pubkey,
    system_instruction::transfer,
};
use solana_program_test::{processor, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentLevel,
    epoch_schedule::EpochSchedule,
    native_token::lamports_to_sol,
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_stake_pool::find_withdraw_authority_program_address;

use super::{
    generated_switchboard_accounts::get_switchboard_accounts,
    restaking_client::NcnRoot,
    stake_pool_client::{PoolRoot, StakePoolClient},
    tip_distribution_client::TipDistributionClient,
    tip_router_client::TipRouterClient,
};
use crate::fixtures::{
    restaking_client::{OperatorRoot, RestakingProgramClient},
    vault_client::{VaultProgramClient, VaultRoot},
    TestResult,
};

pub struct TestNcn {
    pub ncn_root: NcnRoot,
    pub operators: Vec<OperatorRoot>,
    pub vaults: Vec<VaultRoot>,
}

//TODO implement for more fine-grained relationship control
#[allow(dead_code)]

pub struct TestNcnNode {
    pub ncn_root: NcnRoot,
    pub operator_root: OperatorRoot,
    pub vault_root: VaultRoot,

    pub ncn_vault_connected: bool,
    pub operator_vault_connected: bool,
    pub delegation: u64,
}

pub struct TestBuilder {
    context: ProgramTestContext,
    stake_pool_keypair: Keypair,
}

impl Debug for TestBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestBuilder",)
    }
}

pub fn token_mint_account(withdraw_authority: &Pubkey) -> Account {
    let account = spl_token::state::Mint {
        mint_authority: solana_sdk::program_option::COption::Some(*withdraw_authority),
        supply: 0,
        decimals: 9,
        is_initialized: true,
        freeze_authority: solana_sdk::program_option::COption::None,
    };

    let mut data = [0; 82];

    spl_token::state::Mint::pack(account, &mut data).unwrap();

    Account {
        lamports: 1000000000,
        owner: spl_token::id(),
        executable: false,
        rent_epoch: 0,
        data: data.to_vec(),
    }
}

impl TestBuilder {
    pub async fn new() -> Self {
        let run_as_bpf = std::env::vars().any(|(key, _)| key.eq("SBF_OUT_DIR"));

        let mut program_test = if run_as_bpf {
            let mut program_test = ProgramTest::new(
                "jito_tip_router_program",
                jito_tip_router_program::id(),
                None,
            );
            program_test.add_program("jito_vault_program", jito_vault_program::id(), None);
            program_test.add_program("jito_restaking_program", jito_restaking_program::id(), None);
            program_test.add_program("spl_stake_pool", spl_stake_pool::id(), None);

            // Tests that invoke this program should be in the "bpf" module so we can run them separately with the bpf vm.
            // Anchor programs do not expose a compatible entrypoint for solana_program_test::processor!
            program_test.add_program("jito_tip_distribution", jito_tip_distribution::ID, None);

            program_test
        } else {
            let mut program_test = ProgramTest::new(
                "jito_tip_router_program",
                jito_tip_router_program::id(),
                processor!(jito_tip_router_program::process_instruction),
            );
            program_test.add_program(
                "jito_vault_program",
                jito_vault_program::id(),
                processor!(jito_vault_program::process_instruction),
            );
            program_test.add_program(
                "jito_restaking_program",
                jito_restaking_program::id(),
                processor!(jito_restaking_program::process_instruction),
            );
            program_test.add_program(
                "spl_stake_pool",
                spl_stake_pool::id(),
                processor!(spl_stake_pool::processor::Processor::process),
            );
            program_test
        };

        // Add switchboard account
        {
            let switchboard_accounts = get_switchboard_accounts();

            for (address, account) in switchboard_accounts.iter() {
                program_test.add_account(*address, account.clone());
            }
        }

        // Stake pool keypair is needed to create the pool, and JitoSOL mint authority is based on this keypair
        let stake_pool_keypair = Keypair::new();
        let jitosol_mint_authority = find_withdraw_authority_program_address(
            &spl_stake_pool::id(),
            &stake_pool_keypair.pubkey(),
        );
        // Needed to create JitoSOL mint since we don't have access to the original keypair in the tests
        program_test.add_account(JITOSOL_MINT, token_mint_account(&jitosol_mint_authority.0));

        Self {
            context: program_test.start_with_context().await,
            stake_pool_keypair,
        }
    }

    pub async fn get_associated_token_account(
        &mut self,
        wallet: &Pubkey,
        mint: &Pubkey,
    ) -> Result<Option<spl_token::state::Account>, BanksClientError> {
        let ata = spl_associated_token_account::get_associated_token_address(wallet, mint);
        self.get_account(&ata).await.map(|opt_acct| {
            opt_acct.map(|acct| spl_token::state::Account::unpack(&acct.data).unwrap())
        })
    }

    pub async fn get_account(
        &mut self,
        address: &Pubkey,
    ) -> Result<Option<Account>, BanksClientError> {
        self.context.banks_client.get_account(*address).await
    }

    pub async fn warp_slot_incremental(
        &mut self,
        incremental_slots: u64,
    ) -> Result<(), BanksClientError> {
        let clock: Clock = self.context.banks_client.get_sysvar().await?;
        self.context
            .warp_to_slot(clock.slot.checked_add(incremental_slots).unwrap())
            .map_err(|_| BanksClientError::ClientError("failed to warp slot"))?;
        Ok(())
    }

    pub async fn set_account(&mut self, address: Pubkey, account: Account) {
        self.context
            .borrow_mut()
            .set_account(&address, &account.into())
    }

    pub async fn clock(&mut self) -> Clock {
        self.context.banks_client.get_sysvar().await.unwrap()
    }

    pub async fn epoch_schedule(&mut self) -> EpochSchedule {
        self.context.banks_client.get_sysvar().await.unwrap()
    }

    pub fn tip_router_client(&self) -> TipRouterClient {
        TipRouterClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    pub fn restaking_program_client(&self) -> RestakingProgramClient {
        RestakingProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    pub fn vault_client(&self) -> VaultProgramClient {
        VaultProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    pub fn vault_program_client(&self) -> VaultProgramClient {
        VaultProgramClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    pub fn tip_distribution_client(&self) -> TipDistributionClient {
        TipDistributionClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
        )
    }

    pub fn stake_pool_client(&self) -> StakePoolClient {
        StakePoolClient::new(
            self.context.banks_client.clone(),
            self.context.payer.insecure_clone(),
            self.stake_pool_keypair.insecure_clone(),
        )
    }

    #[allow(dead_code)]
    pub async fn transfer(&mut self, to: &Pubkey, sol: f64) -> Result<(), BanksClientError> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        let lamports = sol_to_lamports(sol);
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(&self.context.payer.pubkey(), to, lamports)],
                    Some(&self.context.payer.pubkey()),
                    &[&self.context.payer],
                    blockhash,
                ),
                CommitmentLevel::Processed,
            )
            .await
    }

    pub async fn setup_ncn(&mut self) -> TestResult<NcnRoot> {
        let mut restaking_program_client = self.restaking_program_client();
        let mut vault_program_client = self.vault_program_client();

        vault_program_client.do_initialize_config().await?;
        restaking_program_client.do_initialize_config().await?;
        let ncn_root = restaking_program_client
            .do_initialize_ncn(Some(self.context.payer.insecure_clone()))
            .await?;

        Ok(ncn_root)
    }

    // 1. Setup NCN
    pub async fn create_test_ncn(&mut self) -> TestResult<TestNcn> {
        let mut restaking_program_client = self.restaking_program_client();
        let mut vault_program_client = self.vault_program_client();
        let mut tip_router_client = self.tip_router_client();

        vault_program_client.do_initialize_config().await?;
        restaking_program_client.do_initialize_config().await?;
        let ncn_root = restaking_program_client
            .do_initialize_ncn(Some(self.context.payer.insecure_clone()))
            .await?;

        tip_router_client.setup_tip_router(&ncn_root).await?;

        tip_router_client
            .do_set_config_fees(
                Some(300),
                None,
                Some(self.context.payer.pubkey()),
                Some(270),
                None,
                Some(15),
                &ncn_root,
            )
            .await?;

        Ok(TestNcn {
            ncn_root: ncn_root.clone(),
            operators: vec![],
            vaults: vec![],
        })
    }

    // 1a.
    pub async fn create_custom_test_ncn(
        &mut self,
        base_fee_bps: u16,
        ncn_fee_bps: u16,
    ) -> TestResult<TestNcn> {
        let mut restaking_program_client = self.restaking_program_client();
        let mut vault_program_client = self.vault_program_client();
        let mut tip_router_client = self.tip_router_client();

        vault_program_client.do_initialize_config().await?;
        restaking_program_client.do_initialize_config().await?;
        let ncn_root = restaking_program_client
            .do_initialize_ncn(Some(self.context.payer.insecure_clone()))
            .await?;

        tip_router_client.setup_tip_router(&ncn_root).await?;

        tip_router_client
            .do_set_config_fees(
                Some(300),
                None,
                Some(self.context.payer.pubkey()),
                Some(base_fee_bps),
                None,
                Some(ncn_fee_bps),
                &ncn_root,
            )
            .await?;

        Ok(TestNcn {
            ncn_root: ncn_root.clone(),
            operators: vec![],
            vaults: vec![],
        })
    }

    // 2. Setup Operators
    pub async fn add_operators_to_test_ncn(
        &mut self,
        test_ncn: &mut TestNcn,
        operator_count: usize,
        operator_fees_bps: Option<u16>,
    ) -> TestResult<()> {
        let mut restaking_program_client = self.restaking_program_client();

        for _ in 0..operator_count {
            let operator_root = restaking_program_client
                .do_initialize_operator(operator_fees_bps)
                .await?;

            // ncn <> operator
            restaking_program_client
                .do_initialize_ncn_operator_state(
                    &test_ncn.ncn_root,
                    &operator_root.operator_pubkey,
                )
                .await?;
            self.warp_slot_incremental(1).await.unwrap();
            restaking_program_client
                .do_ncn_warmup_operator(&test_ncn.ncn_root, &operator_root.operator_pubkey)
                .await?;
            restaking_program_client
                .do_operator_warmup_ncn(&operator_root, &test_ncn.ncn_root.ncn_pubkey)
                .await?;

            test_ncn.operators.push(operator_root);
        }

        Ok(())
    }

    // 3. Setup Vaults
    pub async fn add_vaults_to_test_ncn(
        &mut self,
        test_ncn: &mut TestNcn,
        vault_count: usize,
    ) -> TestResult<()> {
        let mut vault_program_client = self.vault_program_client();
        let mut restaking_program_client = self.restaking_program_client();

        const DEPOSIT_FEE_BPS: u16 = 0;
        const WITHDRAWAL_FEE_BPS: u16 = 0;
        const REWARD_FEE_BPS: u16 = 0;
        const MINT_AMOUNT: u64 = 1_000_000;

        for _ in 0..vault_count {
            let vault_root = vault_program_client
                .do_initialize_vault(
                    DEPOSIT_FEE_BPS,
                    WITHDRAWAL_FEE_BPS,
                    REWARD_FEE_BPS,
                    9,
                    &self.context.payer.pubkey(),
                )
                .await?;

            // vault <> ncn
            restaking_program_client
                .do_initialize_ncn_vault_ticket(&test_ncn.ncn_root, &vault_root.vault_pubkey)
                .await?;
            self.warp_slot_incremental(1).await.unwrap();
            restaking_program_client
                .do_warmup_ncn_vault_ticket(&test_ncn.ncn_root, &vault_root.vault_pubkey)
                .await?;
            vault_program_client
                .do_initialize_vault_ncn_ticket(&vault_root, &test_ncn.ncn_root.ncn_pubkey)
                .await?;
            self.warp_slot_incremental(1).await.unwrap();
            vault_program_client
                .do_warmup_vault_ncn_ticket(&vault_root, &test_ncn.ncn_root.ncn_pubkey)
                .await?;

            for operator_root in test_ncn.operators.iter() {
                // vault <> operator
                restaking_program_client
                    .do_initialize_operator_vault_ticket(operator_root, &vault_root.vault_pubkey)
                    .await?;
                self.warp_slot_incremental(1).await.unwrap();
                restaking_program_client
                    .do_warmup_operator_vault_ticket(operator_root, &vault_root.vault_pubkey)
                    .await?;
                vault_program_client
                    .do_initialize_vault_operator_delegation(
                        &vault_root,
                        &operator_root.operator_pubkey,
                    )
                    .await?;
            }

            let depositor_keypair = self.context.payer.insecure_clone();
            let depositor = depositor_keypair.pubkey();
            vault_program_client
                .configure_depositor(&vault_root, &depositor, MINT_AMOUNT)
                .await?;
            vault_program_client
                .do_mint_to(&vault_root, &depositor_keypair, MINT_AMOUNT, MINT_AMOUNT)
                .await
                .unwrap();

            test_ncn.vaults.push(vault_root);
        }

        Ok(())
    }

    // 4. Setup Delegations
    pub async fn add_delegation_in_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
        delegation_amount: usize,
    ) -> TestResult<()> {
        let mut vault_program_client = self.vault_program_client();

        for vault_root in test_ncn.vaults.iter() {
            for operator_root in test_ncn.operators.iter() {
                vault_program_client
                    .do_add_delegation(
                        vault_root,
                        &operator_root.operator_pubkey,
                        delegation_amount as u64,
                    )
                    .await
                    .unwrap();
            }
        }

        Ok(())
    }

    // 5. Setup Tracked Mints
    pub async fn add_vault_registry_to_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();
        let mut restaking_client = self.restaking_program_client();
        let mut vault_client = self.vault_program_client();

        let restaking_config_address =
            Config::find_program_address(&jito_restaking_program::id()).0;
        let restaking_config = restaking_client
            .get_config(&restaking_config_address)
            .await?;

        let epoch_length = restaking_config.epoch_length();

        self.warp_slot_incremental(epoch_length * 2).await.unwrap();

        for vault in test_ncn.vaults.iter() {
            let ncn = test_ncn.ncn_root.ncn_pubkey;
            let vault = vault.vault_pubkey;

            let operators = test_ncn
                .operators
                .iter()
                .map(|operator| operator.operator_pubkey)
                .collect::<Vec<Pubkey>>();

            vault_client
                .do_full_vault_update(&vault, &operators)
                .await?;

            let st_mint = vault_client.get_vault(&vault).await?.supported_mint;

            let vault_ncn_ticket =
                VaultNcnTicket::find_program_address(&jito_vault_program::id(), &vault, &ncn).0;

            let ncn_vault_ticket =
                NcnVaultTicket::find_program_address(&jito_restaking_program::id(), &ncn, &vault).0;

            tip_router_client
                .do_admin_register_st_mint(
                    ncn,
                    st_mint,
                    NcnFeeGroup::lst(),
                    10_000,
                    Some(JTO_SOL_FEED),
                    None,
                )
                .await?;

            tip_router_client
                .do_register_vault(ncn, vault, vault_ncn_ticket, ncn_vault_ticket)
                .await?;
        }

        Ok(())
    }

    // Intermission: setup just NCN
    pub async fn create_initial_test_ncn(
        &mut self,
        operator_count: usize,
        vault_count: usize,
        operator_fees_bps: Option<u16>,
    ) -> TestResult<TestNcn> {
        let mut test_ncn = self.create_test_ncn().await?;
        self.add_operators_to_test_ncn(&mut test_ncn, operator_count, operator_fees_bps)
            .await?;
        self.add_vaults_to_test_ncn(&mut test_ncn, vault_count)
            .await?;
        self.add_delegation_in_test_ncn(&test_ncn, 100).await?;
        self.add_vault_registry_to_test_ncn(&test_ncn).await?;

        Ok(test_ncn)
    }

    pub async fn create_custom_initial_test_ncn(
        &mut self,
        operator_count: usize,
        vault_count: usize,
        operator_fees_bps: u16,
        base_fee_bps: u16,
        ncn_fee_bps: u16,
    ) -> TestResult<TestNcn> {
        let mut test_ncn = self
            .create_custom_test_ncn(base_fee_bps, ncn_fee_bps)
            .await?;
        self.add_operators_to_test_ncn(&mut test_ncn, operator_count, Some(operator_fees_bps))
            .await?;
        self.add_vaults_to_test_ncn(&mut test_ncn, vault_count)
            .await?;
        self.add_delegation_in_test_ncn(&test_ncn, 100).await?;
        self.add_vault_registry_to_test_ncn(&test_ncn).await?;

        Ok(test_ncn)
    }

    // 6-1. Admin Set weights
    pub async fn add_epoch_state_for_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        // Not sure if this is needed
        self.warp_slot_incremental(1000).await?;

        let clock = self.clock().await;
        let epoch = clock.epoch;
        tip_router_client
            .do_full_initialize_epoch_state(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        Ok(())
    }

    // 6a. Admin Set weights
    pub async fn add_admin_weights_for_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();
        let mut vault_client = self.vault_program_client();

        const WEIGHT: u128 = 100;

        let clock = self.clock().await;
        let epoch = clock.epoch;
        tip_router_client
            .do_full_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        for vault_root in test_ncn.vaults.iter() {
            let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

            let st_mint = vault.supported_mint;

            tip_router_client
                .do_admin_set_weight(test_ncn.ncn_root.ncn_pubkey, epoch, st_mint, WEIGHT)
                .await?;
        }

        Ok(())
    }

    // 6b. Switchboard Set weights
    pub async fn add_switchboard_weights_for_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();
        let mut vault_client = self.vault_program_client();

        // Not sure if this is needed
        self.warp_slot_incremental(1000).await?;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let clock = self.clock().await;
        let epoch = clock.epoch;
        tip_router_client
            .do_full_initialize_weight_table(ncn, epoch)
            .await?;

        for vault_root in test_ncn.vaults.iter() {
            let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

            let st_mint = vault.supported_mint;

            tip_router_client
                .do_switchboard_set_weight(ncn, epoch, st_mint)
                .await?;
        }

        Ok(())
    }

    // 7. Create Epoch Snapshot
    pub async fn add_epoch_snapshot_to_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let clock = self.clock().await;
        let epoch = clock.epoch;

        tip_router_client
            .do_initialize_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, epoch)
            .await?;

        Ok(())
    }

    // 8. Create all operator snapshots
    pub async fn add_operator_snapshots_to_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let clock = self.clock().await;
        let epoch = clock.epoch;

        let ncn = test_ncn.ncn_root.ncn_pubkey;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            tip_router_client
                .do_full_initialize_operator_snapshot(operator, ncn, epoch)
                .await?;
        }

        Ok(())
    }

    // 9. Take all VaultOperatorDelegation snapshots
    pub async fn add_vault_operator_delegation_snapshots_to_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let clock = self.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;
            for vault_root in test_ncn.vaults.iter() {
                let vault = vault_root.vault_pubkey;

                tip_router_client
                    .do_snapshot_vault_operator_delegation(vault, operator, ncn, epoch)
                    .await?;
            }
        }

        Ok(())
    }

    // Intermission 2 - all snapshots are taken
    pub async fn snapshot_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        self.add_epoch_state_for_test_ncn(test_ncn).await?;
        self.add_admin_weights_for_test_ncn(test_ncn).await?;
        self.add_epoch_snapshot_to_test_ncn(test_ncn).await?;
        self.add_operator_snapshots_to_test_ncn(test_ncn).await?;
        self.add_vault_operator_delegation_snapshots_to_test_ncn(test_ncn)
            .await?;

        Ok(())
    }

    // 10 - Initialize Ballot Box
    pub async fn add_ballot_box_to_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let clock = self.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        tip_router_client
            .do_full_initialize_ballot_box(ncn, epoch)
            .await?;

        Ok(())
    }

    // 11 - Cast all votes
    pub async fn cast_votes_for_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let clock = self.clock().await;
        let epoch = clock.epoch;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        let meta_merkle_root = [1u8; 32];

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            tip_router_client
                .do_cast_vote(
                    ncn,
                    operator,
                    &operator_root.operator_admin,
                    meta_merkle_root,
                    epoch,
                )
                .await?;
        }

        Ok(())
    }

    // Intermission 3 - come to consensus
    pub async fn vote_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        self.add_ballot_box_to_test_ncn(test_ncn).await?;
        self.cast_votes_for_test_ncn(test_ncn).await?;

        Ok(())
    }

    // 12 - Create Routers
    pub async fn add_routers_for_tests_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let ncn: Pubkey = test_ncn.ncn_root.ncn_pubkey;
        let clock = self.clock().await;
        let epoch = clock.epoch;

        tip_router_client
            .do_full_initialize_base_reward_router(ncn, epoch)
            .await?;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            for group in NcnFeeGroup::all_groups().iter() {
                tip_router_client
                    .do_initialize_ncn_reward_router(*group, ncn, operator, epoch)
                    .await?;
            }
        }

        Ok(())
    }

    // 13 - Route base rewards
    pub async fn route_in_base_rewards_for_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
        rewards: u64,
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = self.clock().await.epoch;

        let base_reward_receiver =
            BaseRewardReceiver::find_program_address(&jito_tip_router_program::id(), &ncn, epoch).0;

        let sol_rewards = lamports_to_sol(rewards);

        // send rewards to the base reward router
        tip_router_client
            .airdrop(&base_reward_receiver, sol_rewards)
            .await?;

        // route rewards
        tip_router_client.do_route_base_rewards(ncn, epoch).await?;

        let base_reward_router = tip_router_client.get_base_reward_router(ncn, epoch).await?;

        // Base Rewards
        for group in BaseFeeGroup::all_groups().iter() {
            let rewards = base_reward_router.base_fee_group_reward(*group).unwrap();

            if rewards == 0 {
                continue;
            }

            tip_router_client
                .do_distribute_base_rewards(*group, ncn, epoch, pool_root)
                .await?;
        }

        // Ncn
        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            let operator_route = base_reward_router.ncn_fee_group_reward_route(&operator);

            if let Ok(operator_route) = operator_route {
                for group in NcnFeeGroup::all_groups().iter() {
                    let rewards = operator_route.rewards(*group).unwrap();

                    if rewards == 0 {
                        continue;
                    }

                    //TODO remove
                    // let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
                    //     &jito_tip_router_program::id(),
                    //     *group,
                    //     &operator,
                    //     &ncn,
                    //     epoch,
                    // );
                    // let sol_rewards = lamports_to_sol(rewards);
                    // tip_router_client
                    //     .airdrop(&ncn_reward_receiver, sol_rewards)
                    //     .await?;

                    tip_router_client
                        .do_distribute_base_ncn_reward_route(*group, operator, ncn, epoch)
                        .await?;
                }
            }
        }

        Ok(())
    }

    // 14 - Route ncn rewards
    pub async fn route_in_ncn_rewards_for_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let ncn = test_ncn.ncn_root.ncn_pubkey;
        let epoch = self.clock().await.epoch;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            for group in NcnFeeGroup::all_groups().iter() {
                tip_router_client
                    .do_route_ncn_rewards(*group, ncn, operator, epoch)
                    .await?;

                let ncn_reward_router = tip_router_client
                    .get_ncn_reward_router(*group, operator, ncn, epoch)
                    .await?;

                let operator_rewards = ncn_reward_router.operator_rewards();

                if operator_rewards > 0 {
                    tip_router_client
                        .do_distribute_ncn_operator_rewards(*group, operator, ncn, epoch, pool_root)
                        .await?;
                }

                for vault_root in test_ncn.vaults.iter() {
                    let vault = vault_root.vault_pubkey;

                    let vault_reward_route = ncn_reward_router.vault_reward_route(&vault);

                    if let Ok(vault_reward_route) = vault_reward_route {
                        let vault_rewards = vault_reward_route.rewards();

                        if vault_rewards > 0 {
                            tip_router_client
                                .do_distribute_ncn_vault_rewards(
                                    *group, vault, operator, ncn, epoch, pool_root,
                                )
                                .await?;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // Intermission 4 - route rewards
    pub async fn reward_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
        rewards: u64,
        pool_root: &PoolRoot,
    ) -> TestResult<()> {
        let mut stake_pool_client = self.stake_pool_client();

        self.add_routers_for_tests_ncn(test_ncn).await?;

        stake_pool_client
            .update_stake_pool_balance(pool_root)
            .await?;

        self.route_in_base_rewards_for_test_ncn(test_ncn, rewards, pool_root)
            .await?;
        self.route_in_ncn_rewards_for_test_ncn(test_ncn, pool_root)
            .await?;

        Ok(())
    }
}

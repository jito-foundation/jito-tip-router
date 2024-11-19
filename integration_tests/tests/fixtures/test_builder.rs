use std::fmt::{Debug, Formatter};

use jito_restaking_core::{config::Config, ncn_vault_ticket::NcnVaultTicket};
use jito_vault_core::vault_ncn_ticket::VaultNcnTicket;
use solana_program::{
    clock::Clock, native_token::sol_to_lamports, pubkey::Pubkey, system_instruction::transfer,
};
use solana_program_test::{processor, BanksClientError, ProgramTest, ProgramTestContext};
use solana_sdk::{commitment_config::CommitmentLevel, signature::Signer, transaction::Transaction};

use super::{restaking_client::NcnRoot, tip_router_client::TipRouterClient};
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
}

impl Debug for TestBuilder {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "TestBuilder",)
    }
}

impl TestBuilder {
    pub async fn new() -> Self {
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
        program_test.prefer_bpf(true);

        Self {
            context: program_test.start_with_context().await,
        }
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

    pub async fn clock(&mut self) -> Clock {
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

    #[allow(dead_code)]
    pub async fn transfer(&mut self, to: &Pubkey, sol: f64) -> Result<(), BanksClientError> {
        let blockhash = self.context.banks_client.get_latest_blockhash().await?;
        self.context
            .banks_client
            .process_transaction_with_preflight_and_commitment(
                Transaction::new_signed_with_payer(
                    &[transfer(
                        &self.context.payer.pubkey(),
                        to,
                        sol_to_lamports(sol),
                    )],
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
    ) -> TestResult<()> {
        let mut restaking_program_client = self.restaking_program_client();

        for _ in 0..operator_count {
            let operator_root = restaking_program_client.do_initialize_operator().await?;

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
                    .do_initialize_operator_vault_ticket(&operator_root, &vault_root.vault_pubkey)
                    .await?;
                self.warp_slot_incremental(1).await.unwrap();
                restaking_program_client
                    .do_warmup_operator_vault_ticket(&operator_root, &vault_root.vault_pubkey)
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
                        &vault_root,
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
    pub async fn add_tracked_mints_to_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
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

            let vault_ncn_ticket =
                VaultNcnTicket::find_program_address(&jito_vault_program::id(), &vault, &ncn).0;

            let ncn_vault_ticket =
                NcnVaultTicket::find_program_address(&jito_restaking_program::id(), &ncn, &vault).0;

            tip_router_client
                .do_register_mint(ncn, vault, vault_ncn_ticket, ncn_vault_ticket)
                .await?;
        }

        Ok(())
    }

    // Intermission: setup just NCN
    pub async fn create_initial_test_ncn(
        &mut self,
        operator_count: usize,
        vault_count: usize,
    ) -> TestResult<TestNcn> {
        let mut test_ncn = self.create_test_ncn().await?;
        self.add_operators_to_test_ncn(&mut test_ncn, operator_count)
            .await?;
        self.add_vaults_to_test_ncn(&mut test_ncn, vault_count)
            .await?;
        self.add_delegation_in_test_ncn(&test_ncn, 100).await?;
        self.add_tracked_mints_to_test_ncn(&test_ncn).await?;

        Ok(test_ncn)
    }

    // 6. Set weights
    pub async fn add_weights_for_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();
        let mut vault_client = self.vault_program_client();

        const WEIGHT: u128 = 100;

        // Not sure if this is needed
        self.warp_slot_incremental(1000).await?;

        let slot = self.clock().await.slot;
        tip_router_client
            .do_initialize_weight_table(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        for vault_root in test_ncn.vaults.iter() {
            let vault = vault_client.get_vault(&vault_root.vault_pubkey).await?;

            let mint = vault.supported_mint;

            tip_router_client
                .do_admin_update_weight_table(test_ncn.ncn_root.ncn_pubkey, slot, mint, WEIGHT)
                .await?;
        }

        Ok(())
    }

    // 7. Create Epoch Snapshot
    pub async fn add_epoch_snapshot_to_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let slot = self.clock().await.slot;

        tip_router_client
            .do_initialize_epoch_snapshot(test_ncn.ncn_root.ncn_pubkey, slot)
            .await?;

        Ok(())
    }

    // 8. Create all operator snapshots
    pub async fn add_operator_snapshots_to_test_ncn(
        &mut self,
        test_ncn: &TestNcn,
    ) -> TestResult<()> {
        let mut tip_router_client = self.tip_router_client();

        let slot = self.clock().await.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;

            tip_router_client
                .do_initalize_operator_snapshot(operator, ncn, slot)
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

        let slot = self.clock().await.slot;
        let ncn = test_ncn.ncn_root.ncn_pubkey;

        for operator_root in test_ncn.operators.iter() {
            let operator = operator_root.operator_pubkey;
            for vault_root in test_ncn.vaults.iter() {
                let vault = vault_root.vault_pubkey;

                tip_router_client
                    .do_snapshot_vault_operator_delegation(vault, operator, ncn, slot)
                    .await?;
            }
        }

        Ok(())
    }

    // Intermission 2 - all snapshots are taken
    pub async fn snapshot_test_ncn(&mut self, test_ncn: &TestNcn) -> TestResult<()> {
        self.add_weights_for_test_ncn(&test_ncn).await?;
        self.add_epoch_snapshot_to_test_ncn(&test_ncn).await?;
        self.add_operator_snapshots_to_test_ncn(&test_ncn).await?;
        self.add_vault_operator_delegation_snapshots_to_test_ncn(&test_ncn)
            .await?;

        Ok(())
    }
}

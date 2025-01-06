use std::time::Duration;

use crate::handler::CliHandler;
use anyhow::Result;
use jito_restaking_client::instructions::{
    InitializeNcnBuilder, InitializeNcnOperatorStateBuilder, InitializeOperatorBuilder,
    NcnWarmupOperatorBuilder, OperatorWarmupNcnBuilder,
};
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    operator::Operator,
};
use log::info;
use solana_rpc_client::rpc_client::SerializableTransaction;

use solana_sdk::{
    compute_budget::ComputeBudgetInstruction, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use tokio::time::sleep;

// --------------------- RESTAKING -------------------------
pub async fn create_test_ncn(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let base = Keypair::new();
    let (ncn, _, _) = Ncn::find_program_address(&handler.restaking_program_id, &base.pubkey());

    let (config, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);

    let mut ix_builder = InitializeNcnBuilder::new();
    ix_builder
        .config(config)
        .admin(keypair.pubkey())
        .base(base.pubkey())
        .ncn(ncn)
        .instruction();

    let blockhash = client.get_latest_blockhash().await?;
    let tx = Transaction::new_signed_with_payer(
        &[ix_builder.instruction()],
        Some(&keypair.pubkey()),
        &[keypair, &base],
        blockhash,
    );

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        info!("Error: {:?}", e);
        return Ok(());
    }

    info!(
        "\n\n---------- CREATED TEST NCN ----------\nSignature: {:?} \nBase: {:?}\n{:?}\nNCN: {:?}\n",
        tx.get_signature(),
        base.pubkey(),
        base.to_bytes(),
        ncn
    );

    Ok(())
}

pub async fn create_and_add_test_operator(
    handler: &CliHandler,
    operator_fee_bps: u16,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let base = Keypair::new();
    let (operator, _, _) =
        Operator::find_program_address(&handler.restaking_program_id, &base.pubkey());

    let (ncn_operator_state, _, _) =
        NcnOperatorState::find_program_address(&handler.restaking_program_id, &ncn, &operator);

    let (config, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);

    // -------------- Initialize Operator --------------
    let initalize_operator_ix = InitializeOperatorBuilder::new()
        .config(config)
        .admin(keypair.pubkey())
        .base(base.pubkey())
        .operator(operator)
        .operator_fee_bps(operator_fee_bps)
        .instruction();

    let initialize_ncn_operator_state_ix = InitializeNcnOperatorStateBuilder::new()
        .config(config)
        .payer(keypair.pubkey())
        .admin(keypair.pubkey())
        .operator(operator)
        .ncn(ncn)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let ncn_warmup_operator_ix = NcnWarmupOperatorBuilder::new()
        .config(config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .operator(operator)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let operator_warmup_ncn_ix = OperatorWarmupNcnBuilder::new()
        .config(config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .operator(operator)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let blockhash = client.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            initalize_operator_ix,
            initialize_ncn_operator_state_ix,
        ],
        Some(&keypair.pubkey()),
        &[keypair, &base],
        blockhash,
    );

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        info!("Error: {:?}", e);
        return Ok(());
    }

    info!(
        "\n\n---------- CREATED TEST OPERATOR ----------\nSignature: {:?} \nBase: {:?}\n{:?}\nOperator: {:?}\nNCN: {:?}",
        tx.get_signature(),
        base.pubkey(),
        base.to_bytes(),
        operator,
        ncn
    );

    sleep(Duration::from_millis(1000)).await;

    let blockhash = client.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            ncn_warmup_operator_ix,
            operator_warmup_ncn_ix,
        ],
        Some(&keypair.pubkey()),
        &[keypair],
        blockhash,
    );

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        info!("Error: {:?}", e);
        return Ok(());
    }

    info!(
        "\n\n---------- ADDED TEST OPERATOR TO NCN ----------\nSignature: {:?}\n{:?}\nOperator: {:?}\nNCN: {:?}",
        tx.get_signature(),
        base.pubkey(),
        operator,
        ncn
    );

    Ok(())
}

pub async fn create_and_add_test_vault(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let base = Keypair::new();
    let (operator, _, _) =
        Operator::find_program_address(&handler.restaking_program_id, &base.pubkey());

    let (ncn_operator_state, _, _) =
        NcnOperatorState::find_program_address(&handler.restaking_program_id, &ncn, &operator);

    let (config, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);

    // -------------- Initialize Vault --------------
    // let initalize_operator_ix = InitializeOperatorBuilder::new()
    //     .config(config)
    //     .admin(keypair.pubkey())
    //     .base(base.pubkey())
    //     .operator(operator)
    //     .operator_fee_bps(operator_fee_bps)
    //     .instruction();

    let initialize_ncn_operator_state_ix = InitializeNcnOperatorStateBuilder::new()
        .config(config)
        .payer(keypair.pubkey())
        .admin(keypair.pubkey())
        .operator(operator)
        .ncn(ncn)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let ncn_warmup_operator_ix = NcnWarmupOperatorBuilder::new()
        .config(config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .operator(operator)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let operator_warmup_ncn_ix = OperatorWarmupNcnBuilder::new()
        .config(config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .operator(operator)
        .ncn_operator_state(ncn_operator_state)
        .instruction();

    let blockhash = client.get_latest_blockhash().await?;

    let tx = Transaction::new_signed_with_payer(
        &[
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            // initalize_operator_ix,
            initialize_ncn_operator_state_ix,
        ],
        Some(&keypair.pubkey()),
        &[keypair, &base],
        blockhash,
    );

    // let result = client.send_and_confirm_transaction(&tx).await;

    // if let Err(e) = result {
    //     info!("Error: {:?}", e);
    //     return Ok(());
    // }

    // info!(
    //     "\n\n---------- CREATED TEST OPERATOR ----------\nSignature: {:?} \nBase: {:?}\n{:?}\nOperator: {:?}\nNCN: {:?}",
    //     tx.get_signature(),
    //     base.pubkey(),
    //     base.to_bytes(),
    //     operator,
    //     ncn
    // );

    // sleep(Duration::from_millis(1000)).await;

    // let blockhash = client.get_latest_blockhash().await?;

    // let tx = Transaction::new_signed_with_payer(
    //     &[
    //         ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
    //         ncn_warmup_operator_ix,
    //         operator_warmup_ncn_ix,
    //     ],
    //     Some(&keypair.pubkey()),
    //     &[keypair],
    //     blockhash,
    // );

    // let result = client.send_and_confirm_transaction(&tx).await;

    // if let Err(e) = result {
    //     info!("Error: {:?}", e);
    //     return Ok(());
    // }

    // info!(
    //     "\n\n---------- ADDED TEST OPERATOR TO NCN ----------\nSignature: {:?} \nBase: {:?}\n{:?}\nOperator: {:?}\nNCN: {:?}",
    //     tx.get_signature(),
    //     base.pubkey(),
    //     base.to_bytes(),
    //     operator,
    //     ncn
    // );

    Ok(())
}

// // 3. Setup Vaults
// pub async fn add_vaults_to_test_ncn(
//     &mut self,
//     test_ncn: &mut TestNcn,
//     vault_count: usize,
// ) -> TestResult<()> {
//     let mut vault_program_client = self.vault_program_client();
//     let mut restaking_program_client = self.restaking_program_client();

//     const DEPOSIT_FEE_BPS: u16 = 0;
//     const WITHDRAWAL_FEE_BPS: u16 = 0;
//     const REWARD_FEE_BPS: u16 = 0;
//     const MINT_AMOUNT: u64 = 1_000_000;

//     for _ in 0..vault_count {
//         let vault_root = vault_program_client
//             .do_initialize_vault(
//                 DEPOSIT_FEE_BPS,
//                 WITHDRAWAL_FEE_BPS,
//                 REWARD_FEE_BPS,
//                 9,
//                 &self.context.payer.pubkey(),
//             )
//             .await?;

//         // vault <> ncn
//         restaking_program_client
//             .do_initialize_ncn_vault_ticket(&test_ncn.ncn_root, &vault_root.vault_pubkey)
//             .await?;
//         self.warp_slot_incremental(1).await.unwrap();
//         restaking_program_client
//             .do_warmup_ncn_vault_ticket(&test_ncn.ncn_root, &vault_root.vault_pubkey)
//             .await?;
//         vault_program_client
//             .do_initialize_vault_ncn_ticket(&vault_root, &test_ncn.ncn_root.ncn_pubkey)
//             .await?;
//         self.warp_slot_incremental(1).await.unwrap();
//         vault_program_client
//             .do_warmup_vault_ncn_ticket(&vault_root, &test_ncn.ncn_root.ncn_pubkey)
//             .await?;

//         for operator_root in test_ncn.operators.iter() {
//             // vault <> operator
//             restaking_program_client
//                 .do_initialize_operator_vault_ticket(operator_root, &vault_root.vault_pubkey)
//                 .await?;
//             self.warp_slot_incremental(1).await.unwrap();
//             restaking_program_client
//                 .do_warmup_operator_vault_ticket(operator_root, &vault_root.vault_pubkey)
//                 .await?;
//             vault_program_client
//                 .do_initialize_vault_operator_delegation(
//                     &vault_root,
//                     &operator_root.operator_pubkey,
//                 )
//                 .await?;
//         }

//         let depositor_keypair = self.context.payer.insecure_clone();
//         let depositor = depositor_keypair.pubkey();
//         vault_program_client
//             .configure_depositor(&vault_root, &depositor, MINT_AMOUNT)
//             .await?;
//         vault_program_client
//             .do_mint_to(&vault_root, &depositor_keypair, MINT_AMOUNT, MINT_AMOUNT)
//             .await
//             .unwrap();

//         test_ncn.vaults.push(vault_root);
//     }

//     Ok(())
// }

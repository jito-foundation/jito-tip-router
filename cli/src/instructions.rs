use std::time::Duration;

use crate::{getters::get_all_operators_in_ncn, handler::CliHandler};
use anyhow::{anyhow, Ok, Result};
use jito_restaking_client::instructions::{
    InitializeNcnBuilder, InitializeNcnOperatorStateBuilder, InitializeNcnVaultTicketBuilder,
    InitializeOperatorBuilder, InitializeOperatorVaultTicketBuilder, NcnWarmupOperatorBuilder,
    OperatorWarmupNcnBuilder, WarmupNcnVaultTicketBuilder, WarmupOperatorVaultTicketBuilder,
};
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
    operator_vault_ticket::OperatorVaultTicket,
};
use jito_tip_router_client::instructions::InitializeConfig;
use jito_vault_client::instructions::{
    AddDelegationBuilder, InitializeVaultBuilder, InitializeVaultNcnTicketBuilder,
    InitializeVaultOperatorDelegationBuilder, MintToBuilder, WarmupVaultNcnTicketBuilder,
};
use jito_vault_core::{
    config::Config as VaultConfig, vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client::rpc_client::SerializableTransaction;

use solana_sdk::{
    instruction::Instruction,
    program_pack::Pack,
    rent::Rent,
    signature::{Keypair, Signature},
    signer::Signer,
    system_instruction::create_account,
    system_program,
    transaction::Transaction,
};
use spl_associated_token_account::get_associated_token_address;
use tokio::time::sleep;

// --------------------- TIP ROUTER ------------------------------

pub async fn create_config(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let base = Keypair::new();
    let (ncn, _, _) = Ncn::find_program_address(&handler.restaking_program_id, &base.pubkey());

    let (config, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);

    // let mut ix_builder = InitializeConfig::new()
    //     .config(config)
    //     .admin(keypair.pubkey())
    //     .base(base.pubkey())
    //     .ncn(ncn)
    //     .instruction();

    // send_and_log_transaction(
    //     &client,
    //     &keypair,
    //     &[ix_builder.instruction()],
    //     &[&base],
    //     "Created Test Ncn",
    //     &[format!("NCN: {:?}", ncn)],
    // )
    // .await?;

    Ok(())
}

//TODO create vault registry
//TODO admin register st mint
//TODO admin register vault
//TODO create weight table
//TODO admin set weight
//TODO set weight
//TODO create epoch snapshot
//TODO create operator snapshot
//TODO snapshot vault operator delegation
//TODO create ballot box
//TODO cast vote
//TODO create base reward router
//TODO create ncn reward router
//TODO route base rewards
//TODO route ncn rewards
//TODO distribute base rewards
//TODO distribute base ncn rewards
//TODO distribute ncn vault rewards
//TODO distribute ncn operator rewards
//TODO admin set tie breaker

// --------------------- NCN SETUP ------------------------------
//TODO create NCN
//TODO create Operator
//TODO add vault to NCN
//TODO add operator to NCN
//TODO remove vault from NCN
//TODO remove operator from NCN

// --------------------- TEST NCN --------------------------------

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

    send_and_log_transaction(
        &client,
        &keypair,
        &[ix_builder.instruction()],
        &[&base],
        "Created Test Ncn",
        &[format!("NCN: {:?}", ncn)],
    )
    .await?;

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

    send_and_log_transaction(
        &client,
        &keypair,
        &[initalize_operator_ix, initialize_ncn_operator_state_ix],
        &[&base],
        "Created Test Operator",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
        ],
    )
    .await?;

    sleep(Duration::from_millis(1000)).await;

    send_and_log_transaction(
        &client,
        &keypair,
        &[ncn_warmup_operator_ix, operator_warmup_ncn_ix],
        &[],
        "Warmed up Operator",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_and_add_test_vault(
    handler: &CliHandler,
    deposit_fee_bps: u16,
    withdrawal_fee_bps: u16,
    reward_fee_bps: u16,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let vrt_mint = Keypair::new();
    let token_mint = Keypair::new();
    let base = Keypair::new();
    let (vault, _, _) = Vault::find_program_address(&handler.vault_program_id, &base.pubkey());

    let (vault_config, _, _) = VaultConfig::find_program_address(&handler.vault_program_id);
    let (restaking_config, _, _) =
        RestakingConfig::find_program_address(&handler.restaking_program_id);

    let all_operators = get_all_operators_in_ncn(handler).await?;

    // -------------- Create Mint -----------------
    let admin_ata = spl_associated_token_account::get_associated_token_address(
        &keypair.pubkey(),
        &token_mint.pubkey(),
    );

    let create_mint_account_ix = create_account(
        &keypair.pubkey(),
        &token_mint.pubkey(),
        Rent::default().minimum_balance(spl_token::state::Mint::LEN),
        spl_token::state::Mint::LEN as u64,
        &handler.token_program_id,
    );
    let initialize_mint_ix = spl_token::instruction::initialize_mint2(
        &handler.token_program_id,
        &token_mint.pubkey(),
        &keypair.pubkey(),
        None,
        9,
    )
    .unwrap();
    let create_admin_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &keypair.pubkey(),
            &keypair.pubkey(),
            &token_mint.pubkey(),
            &handler.token_program_id,
        );
    let mint_to_ix = spl_token::instruction::mint_to(
        &handler.token_program_id,
        &token_mint.pubkey(),
        &admin_ata,
        &keypair.pubkey(),
        &[],
        1_000_000,
    )
    .unwrap();

    send_and_log_transaction(
        &client,
        &keypair,
        &[
            create_mint_account_ix,
            initialize_mint_ix,
            create_admin_ata_ix,
            mint_to_ix,
        ],
        &[&token_mint],
        "Created Test Mint",
        &[format!("Token Mint: {:?}", token_mint.pubkey())],
    )
    .await?;

    // -------------- Initialize Vault --------------
    let initialize_vault_ix = InitializeVaultBuilder::new()
        .config(vault_config)
        .admin(keypair.pubkey())
        .base(base.pubkey())
        .vault(vault)
        .vrt_mint(vrt_mint.pubkey())
        .token_mint(token_mint.pubkey())
        .reward_fee_bps(reward_fee_bps)
        .withdrawal_fee_bps(withdrawal_fee_bps)
        .decimals(9)
        .deposit_fee_bps(deposit_fee_bps)
        .system_program(system_program::id())
        .instruction();

    let create_vault_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &keypair.pubkey(),
            &vault,
            &token_mint.pubkey(),
            &handler.token_program_id,
        );
    let create_admin_vrt_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &keypair.pubkey(),
            &keypair.pubkey(),
            &vrt_mint.pubkey(),
            &handler.token_program_id,
        );
    let create_vault_vrt_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            &keypair.pubkey(),
            &vault,
            &vrt_mint.pubkey(),
            &handler.token_program_id,
        );

    let vault_token_ata = get_associated_token_address(&vault, &token_mint.pubkey());
    let admin_token_ata = get_associated_token_address(&keypair.pubkey(), &token_mint.pubkey());
    let admin_vrt_ata = get_associated_token_address(&keypair.pubkey(), &vrt_mint.pubkey());

    let mint_to_ix = MintToBuilder::new()
        .config(vault_config)
        .vault(vault)
        .vrt_mint(vrt_mint.pubkey())
        .depositor(keypair.pubkey())
        .depositor_token_account(admin_token_ata)
        .depositor_vrt_token_account(admin_vrt_ata)
        .vault_fee_token_account(admin_vrt_ata)
        .vault_token_account(vault_token_ata)
        .amount_in(10_000)
        .min_amount_out(0)
        .instruction();

    send_and_log_transaction(
        &client,
        &keypair,
        &[
            initialize_vault_ix,
            create_vault_ata_ix,
            create_admin_vrt_ata_ix,
            create_vault_vrt_ata_ix,
            mint_to_ix,
        ],
        &[&base, &vrt_mint],
        "Created Test Vault",
        &[
            format!("NCN: {:?}", ncn),
            format!("Vault: {:?}", vault),
            format!("Token Mint: {:?}", token_mint.pubkey()),
            format!("VRT Mint: {:?}", vrt_mint.pubkey()),
        ],
    )
    .await?;

    // -------------- Initialize Vault <> NCN Ticket --------------

    let (ncn_vault_ticket, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, &ncn, &vault);

    let (vault_ncn_ticket, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, &vault, &ncn);

    let initialize_ncn_vault_ticket_ix = InitializeNcnVaultTicketBuilder::new()
        .config(restaking_config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .vault(vault)
        .payer(keypair.pubkey())
        .ncn_vault_ticket(ncn_vault_ticket)
        .instruction();

    let initialize_vault_ncn_ticket_ix = InitializeVaultNcnTicketBuilder::new()
        .config(vault_config)
        .admin(keypair.pubkey())
        .vault(vault)
        .ncn(ncn)
        .payer(keypair.pubkey())
        .vault_ncn_ticket(vault_ncn_ticket)
        .ncn_vault_ticket(ncn_vault_ticket)
        .instruction();

    send_and_log_transaction(
        &client,
        &keypair,
        &[
            initialize_ncn_vault_ticket_ix,
            initialize_vault_ncn_ticket_ix,
        ],
        &[],
        "Initialized Vault and NCN Tickets",
        &[format!("NCN: {:?}", ncn), format!("Vault: {:?}", vault)],
    )
    .await?;

    sleep(Duration::from_millis(1000)).await;

    let warmup_ncn_vault_ticket_ix = WarmupNcnVaultTicketBuilder::new()
        .config(restaking_config)
        .admin(keypair.pubkey())
        .ncn(ncn)
        .vault(vault)
        .ncn_vault_ticket(ncn_vault_ticket)
        .instruction();

    let warmup_vault_ncn_ticket_ix = WarmupVaultNcnTicketBuilder::new()
        .config(vault_config)
        .admin(keypair.pubkey())
        .vault(vault)
        .ncn(ncn)
        .vault_ncn_ticket(vault_ncn_ticket)
        .instruction();

    send_and_log_transaction(
        &client,
        &keypair,
        &[warmup_ncn_vault_ticket_ix, warmup_vault_ncn_ticket_ix],
        &[],
        "Warmed up NCN Vault Tickets",
        &[format!("NCN: {:?}", ncn), format!("Vault: {:?}", vault)],
    )
    .await?;

    for operator in all_operators {
        let (operator_vault_ticket, _, _) = OperatorVaultTicket::find_program_address(
            &handler.restaking_program_id,
            &operator,
            &vault,
        );

        let (vault_operator_delegation, _, _) = VaultOperatorDelegation::find_program_address(
            &handler.vault_program_id,
            &vault,
            &operator,
        );

        let initialize_operator_vault_ticket_ix = InitializeOperatorVaultTicketBuilder::new()
            .config(restaking_config)
            .admin(keypair.pubkey())
            .operator(operator)
            .vault(vault)
            .operator_vault_ticket(operator_vault_ticket)
            .payer(keypair.pubkey())
            .instruction();
        // do_initialize_operator_vault_ticket

        send_and_log_transaction(
            &client,
            &keypair,
            &[initialize_operator_vault_ticket_ix],
            &[],
            "Connected Vault and Operator",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("Vault: {:?}", vault),
            ],
        )
        .await?;

        sleep(Duration::from_millis(1000)).await;

        // do_initialize_vault_operator_delegation
        let warmup_operator_vault_ticket_ix = WarmupOperatorVaultTicketBuilder::new()
            .config(restaking_config)
            .admin(keypair.pubkey())
            .operator(operator)
            .vault(vault)
            .operator_vault_ticket(operator_vault_ticket)
            .instruction();

        let initialize_vault_operator_delegation_ix =
            InitializeVaultOperatorDelegationBuilder::new()
                .config(vault_config)
                .admin(keypair.pubkey())
                .vault(vault)
                .payer(keypair.pubkey())
                .operator(operator)
                .operator_vault_ticket(operator_vault_ticket)
                .vault_operator_delegation(vault_operator_delegation)
                .instruction();

        let delegate_to_operator_ix = AddDelegationBuilder::new()
            .config(vault_config)
            .vault(vault)
            .operator(operator)
            .vault_operator_delegation(vault_operator_delegation)
            .admin(keypair.pubkey())
            .amount(1000)
            .instruction();

        send_and_log_transaction(
            &client,
            &keypair,
            &[
                warmup_operator_vault_ticket_ix,
                initialize_vault_operator_delegation_ix,
                delegate_to_operator_ix,
            ],
            &[],
            "Delegated to Operator",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("Vault: {:?}", vault),
            ],
        )
        .await?;
    }

    Ok(())
}

// --------------------- HELPERS -------------------------

pub async fn send_and_log_transaction(
    client: &RpcClient,
    keypair: &Keypair,
    instructions: &[Instruction],
    signing_keypairs: &[&Keypair],
    title: &str,
    log_items: &[String],
) -> Result<()> {
    let signature = send_transactions(client, keypair, instructions, signing_keypairs).await?;

    log_transaction(title, signature, log_items);

    Ok(())
}

pub async fn send_transactions(
    client: &RpcClient,
    keypair: &Keypair,
    instructions: &[Instruction],
    signing_keypairs: &[&Keypair],
) -> Result<Signature> {
    let blockhash = client.get_latest_blockhash().await?;

    // Create a vector that combines all signing keypairs
    let mut all_signers = vec![keypair];
    all_signers.extend(signing_keypairs.iter());

    let tx = Transaction::new_signed_with_payer(
        instructions,
        Some(&keypair.pubkey()),
        &all_signers, // Pass the reference to the vector of keypair references
        blockhash,
    );

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        return Err(anyhow!("Error: {:?}", e));
    }

    Ok(*tx.get_signature())
}

pub fn log_transaction(title: &str, signature: Signature, log_items: &[String]) {
    let mut log_message = format!(
        "\n\n---------- {} ----------\nSignature: {:?}",
        title, signature
    );

    for item in log_items {
        log_message.push_str(&format!("\n{}", item));
    }

    log_message.push('\n');
    info!("{}", log_message);
}

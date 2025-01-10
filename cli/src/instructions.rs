use std::time::Duration;

use crate::{
    getters::{
        get_account, get_all_operators_in_ncn, get_base_reward_router, get_ncn_reward_router,
        get_vault, get_vault_registry,
    },
    handler::CliHandler,
};
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
use jito_tip_router_client::instructions::{
    AdminRegisterStMintBuilder, AdminSetTieBreakerBuilder, AdminSetWeightBuilder, CastVoteBuilder,
    DistributeBaseNcnRewardRouteBuilder, InitializeBallotBoxBuilder,
    InitializeBaseRewardRouterBuilder, InitializeConfigBuilder as InitializeTipRouterConfigBuilder,
    InitializeEpochSnapshotBuilder, InitializeNcnRewardRouterBuilder,
    InitializeOperatorSnapshotBuilder, InitializeVaultRegistryBuilder,
    InitializeWeightTableBuilder, ReallocBallotBoxBuilder, ReallocBaseRewardRouterBuilder,
    ReallocOperatorSnapshotBuilder, ReallocVaultRegistryBuilder, ReallocWeightTableBuilder,
    RegisterVaultBuilder, RouteBaseRewardsBuilder, RouteNcnRewardsBuilder,
    SnapshotVaultOperatorDelegationBuilder, SwitchboardSetWeightBuilder,
};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as TipRouterConfig,
    constants::MAX_REALLOC_BYTES,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
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

use solana_sdk::{
    compute_budget::ComputeBudgetInstruction,
    instruction::Instruction,
    program_pack::Pack,
    pubkey::Pubkey,
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

#[allow(clippy::too_many_arguments)]
pub async fn admin_create_config(
    handler: &CliHandler,
    epochs_before_stall: u64,
    valid_slots_after_consensus: u64,
    dao_fee_bps: u16,
    block_engine_fee: u16,
    default_ncn_fee_bps: u16,
    fee_wallet: Option<Pubkey>,
    tie_breaker_admin: Option<Pubkey>,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let fee_wallet = fee_wallet.unwrap_or_else(|| keypair.pubkey());
    let tie_breaker_admin = tie_breaker_admin.unwrap_or_else(|| keypair.pubkey());

    let initialize_config_ix = InitializeTipRouterConfigBuilder::new()
        .config(config)
        .ncn_admin(keypair.pubkey())
        .ncn(ncn)
        .epochs_before_stall(epochs_before_stall)
        .valid_slots_after_consensus(valid_slots_after_consensus)
        .dao_fee_bps(dao_fee_bps)
        .block_engine_fee_bps(block_engine_fee)
        .default_ncn_fee_bps(default_ncn_fee_bps)
        .tie_breaker_admin(keypair.pubkey())
        .fee_wallet(fee_wallet)
        .restaking_program(handler.restaking_program_id)
        .instruction();

    let program = client.get_account(&handler.tip_router_program_id).await?;

    info!(
        "\n\n----------------------\nProgram: {:?}\n\nProgram Account:\n{:?}\n\nIX:\n{:?}\n----------------------\n",
        &handler.tip_router_program_id, program, &initialize_config_ix
    );

    send_and_log_transaction(
        client,
        keypair,
        &[initialize_config_ix],
        &[],
        "Created Tip Router Config",
        &[
            format!("NCN: {:?}", ncn),
            format!("Ncn Admin: {:?}", keypair.pubkey()),
            format!("Fee Wallet: {:?}", fee_wallet),
            format!("Tie Breaker Admin: {:?}", tie_breaker_admin),
            format!(
                "Valid Slots After Consensus: {:?}",
                valid_slots_after_consensus
            ),
            format!("DAO Fee BPS: {:?}", dao_fee_bps),
            format!("Block Engine Fee BPS: {:?}", block_engine_fee),
            format!("Default NCN Fee BPS: {:?}", default_ncn_fee_bps),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_vault_registry(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (vault_registry, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, &ncn);

    let vault_registry_account = get_account(handler, &vault_registry).await?;

    // Skip if vault registry already exists
    if vault_registry_account.data.is_empty() {
        let initialize_vault_registry_ix = InitializeVaultRegistryBuilder::new()
            .config(config)
            .payer(keypair.pubkey())
            .ncn(ncn)
            .vault_registry(vault_registry)
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_vault_registry_ix],
            &[],
            "Created Vault Registry",
            &[format!("NCN: {:?}", ncn)],
        )
        .await?;
    }

    // Number of reallocations needed based on VaultRegistry::SIZE
    let num_reallocs = (VaultRegistry::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    let realloc_vault_registry_ix = ReallocVaultRegistryBuilder::new()
        .config(config)
        .vault_registry(vault_registry)
        .ncn(ncn)
        .payer(keypair.pubkey())
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_vault_registry_ix.clone());
    }

    send_and_log_transaction(
        client,
        keypair,
        &realloc_ixs,
        &[],
        "Reallocated Vault Registry",
        &[
            format!("NCN: {:?}", ncn),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_register_st_mint(
    handler: &CliHandler,
    vault: &Pubkey,
    ncn_fee_group: NcnFeeGroup,
    reward_multiplier_bps: u64,
    switchboard_feed: Option<Pubkey>,
    no_feed_weight: Option<u128>,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (vault_registry, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, &ncn);

    let vault_account = get_vault(handler, vault).await?;

    let mut register_st_mint_builder = AdminRegisterStMintBuilder::new();

    register_st_mint_builder
        .config(config)
        .admin(keypair.pubkey())
        .vault_registry(vault_registry)
        .ncn(ncn)
        .st_mint(vault_account.supported_mint)
        .ncn_fee_group(ncn_fee_group.group)
        .restaking_program(handler.restaking_program_id)
        .reward_multiplier_bps(reward_multiplier_bps);

    if let Some(switchboard_feed) = switchboard_feed {
        register_st_mint_builder.switchboard_feed(switchboard_feed);
    }

    if let Some(no_feed_weight) = no_feed_weight {
        register_st_mint_builder.no_feed_weight(no_feed_weight);
    }

    let register_st_mint_ix = register_st_mint_builder.instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[register_st_mint_ix],
        &[],
        "Registered ST Mint",
        &[
            format!("NCN: {:?}", ncn),
            format!("ST Mint: {:?}", vault_account.supported_mint),
            format!("NCN Fee Group: {:?}", ncn_fee_group.group),
            format!("Reward Multiplier BPS: {:?}", reward_multiplier_bps),
            format!(
                "Switchboard Feed: {:?}",
                switchboard_feed.unwrap_or_default()
            ),
            format!("No Feed Weight: {:?}", no_feed_weight.unwrap_or_default()),
        ],
    )
    .await?;

    Ok(())
}

pub async fn register_vault(handler: &CliHandler, vault: &Pubkey) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let vault = *vault;

    let (vault_registry, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, &ncn);

    let (restaking_config, _, _) =
        RestakingConfig::find_program_address(&handler.restaking_program_id);

    let (ncn_vault_ticket, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, &ncn, &vault);

    let (vault_ncn_ticket, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, &vault, &ncn);

    let register_vault_ix = RegisterVaultBuilder::new()
        .vault_registry(vault_registry)
        .vault(vault)
        .ncn(ncn)
        .ncn_vault_ticket(ncn_vault_ticket)
        .restaking_config(restaking_config)
        .restaking_program_id(handler.restaking_program_id)
        .vault_ncn_ticket(vault_ncn_ticket)
        .vault_program_id(handler.vault_program_id)
        .vault_registry(vault_registry)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[register_vault_ix],
        &[],
        "Registered Vault",
        &[format!("NCN: {:?}", ncn), format!("Vault: {:?}", vault)],
    )
    .await?;

    Ok(())
}

pub async fn create_weight_table(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (vault_registry, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, &ncn);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let weight_table_account = get_account(handler, &weight_table).await?;

    // Skip if weight table already exists
    if weight_table_account.data.is_empty() {
        // Initialize weight table
        let initialize_weight_table_ix = InitializeWeightTableBuilder::new()
            .vault_registry(vault_registry)
            .ncn(ncn)
            .weight_table(weight_table)
            .payer(keypair.pubkey())
            .restaking_program(handler.restaking_program_id)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_weight_table_ix],
            &[],
            "Initialized Weight Table",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on WeightTable::SIZE
    let num_reallocs = (WeightTable::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    // Realloc weight table
    let realloc_weight_table_ix = ReallocWeightTableBuilder::new()
        .config(config)
        .weight_table(weight_table)
        .ncn(ncn)
        .vault_registry(vault_registry)
        .epoch(epoch)
        .payer(keypair.pubkey())
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_weight_table_ix.clone());
    }

    send_and_log_transaction(
        client,
        keypair,
        &realloc_ixs,
        &[],
        "Reallocated Weight Table",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_set_weight(handler: &CliHandler, vault: &Pubkey, weight: u128) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let vault_account = get_vault(handler, vault).await?;
    let st_mint = vault_account.supported_mint;

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let admin_set_weight_ix = AdminSetWeightBuilder::new()
        .ncn(ncn)
        .weight_table(weight_table)
        .weight_table_admin(keypair.pubkey())
        .restaking_program(handler.restaking_program_id)
        .st_mint(st_mint)
        .weight(weight)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[admin_set_weight_ix],
        &[],
        "Set Weight",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("ST Mint: {:?}", st_mint),
            format!("Weight: {:?}", weight),
        ],
    )
    .await?;

    Ok(())
}

pub async fn set_weight(handler: &CliHandler, vault: &Pubkey) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let vault_registry = get_vault_registry(handler).await?;
    let vault_account = get_vault(handler, vault).await?;

    let mint_entry = vault_registry.get_mint_entry(&vault_account.supported_mint)?;
    let switchboard_feed = mint_entry.switchboard_feed();

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let set_weight_ix = SwitchboardSetWeightBuilder::new()
        .ncn(ncn)
        .weight_table(weight_table)
        .st_mint(vault_account.supported_mint)
        .switchboard_feed(*switchboard_feed)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[set_weight_ix],
        &[],
        "Set Weight Using Switchboard Feed",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("ST Mint: {:?}", vault_account.supported_mint),
            format!("Switchboard Feed: {:?}", switchboard_feed),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_epoch_snapshot(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let initialize_epoch_snapshot_ix = InitializeEpochSnapshotBuilder::new()
        .config(config)
        .ncn(ncn)
        .weight_table(weight_table)
        .epoch_snapshot(epoch_snapshot)
        .payer(keypair.pubkey())
        .restaking_program(handler.restaking_program_id)
        .system_program(system_program::id())
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[initialize_epoch_snapshot_ix],
        &[],
        "Initialized Epoch Snapshot",
        &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
    )
    .await?;

    Ok(())
}

pub async fn create_operator_snapshot(handler: &CliHandler, operator: &Pubkey) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let operator = *operator;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (ncn_operator_state, _, _) =
        NcnOperatorState::find_program_address(&handler.restaking_program_id, &ncn, &operator);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let operator_snapshot_account = get_account(handler, &operator_snapshot).await?;

    // Skip if operator snapshot already exists
    if operator_snapshot_account.data.is_empty() {
        // Initialize operator snapshot
        let initialize_operator_snapshot_ix = InitializeOperatorSnapshotBuilder::new()
            .config(config)
            .ncn(ncn)
            .operator(operator)
            .ncn_operator_state(ncn_operator_state)
            .epoch_snapshot(epoch_snapshot)
            .operator_snapshot(operator_snapshot)
            .payer(keypair.pubkey())
            .restaking_program(handler.restaking_program_id)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_operator_snapshot_ix],
            &[],
            "Initialized Operator Snapshot",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("Epoch: {:?}", epoch),
            ],
        )
        .await?;
    }

    // Number of reallocations needed based on OperatorSnapshot::SIZE
    let num_reallocs = (OperatorSnapshot::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    // Realloc operator snapshot
    let realloc_operator_snapshot_ix = ReallocOperatorSnapshotBuilder::new()
        .ncn_config(config)
        .restaking_config(RestakingConfig::find_program_address(&handler.restaking_program_id).0)
        .ncn(ncn)
        .operator(operator)
        .ncn_operator_state(ncn_operator_state)
        .epoch_snapshot(epoch_snapshot)
        .operator_snapshot(operator_snapshot)
        .payer(keypair.pubkey())
        .restaking_program(handler.restaking_program_id)
        .system_program(system_program::id())
        .epoch(epoch)
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_operator_snapshot_ix.clone());
    }

    send_and_log_transaction(
        client,
        keypair,
        &realloc_ixs,
        &[],
        "Reallocated Operator Snapshot",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}
pub async fn snapshot_vault_operator_delegation(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let vault = *vault;
    let operator = *operator;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (restaking_config, _, _) =
        RestakingConfig::find_program_address(&handler.restaking_program_id);

    let (vault_ncn_ticket, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, &vault, &ncn);

    let (ncn_vault_ticket, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, &ncn, &vault);

    let (vault_operator_delegation, _, _) =
        VaultOperatorDelegation::find_program_address(&handler.vault_program_id, &vault, &operator);

    let (weight_table, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let snapshot_vault_operator_delegation_ix = SnapshotVaultOperatorDelegationBuilder::new()
        .config(config)
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
        .vault_program(handler.vault_program_id)
        .restaking_program(handler.restaking_program_id)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[snapshot_vault_operator_delegation_ix],
        &[],
        "Snapshotted Vault Operator Delegation",
        &[
            format!("NCN: {:?}", ncn),
            format!("Vault: {:?}", vault),
            format!("Operator: {:?}", operator),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_ballot_box(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (ballot_box, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let ballot_box_account = get_account(handler, &ballot_box).await?;

    // Skip if ballot box already exists
    if ballot_box_account.data.is_empty() {
        // Initialize ballot box
        let initialize_ballot_box_ix = InitializeBallotBoxBuilder::new()
            .config(config)
            .ballot_box(ballot_box)
            .ncn(ncn)
            .epoch(epoch)
            .payer(keypair.pubkey())
            .system_program(system_program::id())
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_ballot_box_ix],
            &[],
            "Initialized Ballot Box",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on BallotBox::SIZE
    let num_reallocs = (BallotBox::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    // Realloc ballot box
    let realloc_ballot_box_ix = ReallocBallotBoxBuilder::new()
        .config(config)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .epoch(epoch)
        .payer(keypair.pubkey())
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_ballot_box_ix.clone());
    }

    send_and_log_transaction(
        client,
        keypair,
        &realloc_ixs,
        &[],
        "Reallocated Ballot Box",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn admin_cast_vote(
    handler: &CliHandler,
    operator: &Pubkey,
    meta_merkle_root: [u8; 32],
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let operator = *operator;

    let (config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (ballot_box, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let cast_vote_ix = CastVoteBuilder::new()
        .config(config)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .epoch_snapshot(epoch_snapshot)
        .operator_snapshot(operator_snapshot)
        .operator(operator)
        .operator_admin(keypair.pubkey())
        .restaking_program(handler.restaking_program_id)
        .meta_merkle_root(meta_merkle_root)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[cast_vote_ix],
        &[],
        "Cast Vote",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("Meta Merkle Root: {:?}", meta_merkle_root),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_base_reward_router(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (base_reward_router, _, _) =
        BaseRewardRouter::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (base_reward_receiver, _, _) =
        BaseRewardReceiver::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let base_reward_router_account = get_account(handler, &base_reward_router).await?;

    // Skip if base reward router already exists
    if base_reward_router_account.data.is_empty() {
        let initialize_base_reward_router_ix = InitializeBaseRewardRouterBuilder::new()
            .ncn(ncn)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .payer(keypair.pubkey())
            .restaking_program(handler.restaking_program_id)
            .system_program(system_program::id())
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_base_reward_router_ix],
            &[],
            "Initialized Base Reward Router",
            &[format!("NCN: {:?}", ncn), format!("Epoch: {:?}", epoch)],
        )
        .await?;
    }

    // Number of reallocations needed based on BaseRewardRouter::SIZE
    let num_reallocs = (BaseRewardRouter::SIZE as f64 / MAX_REALLOC_BYTES as f64).ceil() as u64 - 1;

    let realloc_base_reward_router_ix = ReallocBaseRewardRouterBuilder::new()
        .config(TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn).0)
        .base_reward_router(base_reward_router)
        .ncn(ncn)
        .epoch(epoch)
        .payer(keypair.pubkey())
        .system_program(system_program::id())
        .instruction();

    let mut realloc_ixs = Vec::with_capacity(num_reallocs as usize);
    realloc_ixs.push(ComputeBudgetInstruction::set_compute_unit_limit(1_400_000));
    for _ in 0..num_reallocs {
        realloc_ixs.push(realloc_base_reward_router_ix.clone());
    }

    send_and_log_transaction(
        client,
        keypair,
        &realloc_ixs,
        &[],
        "Reallocated Base Reward Router",
        &[
            format!("NCN: {:?}", ncn),
            format!("Epoch: {:?}", epoch),
            format!("Number of reallocations: {:?}", num_reallocs),
        ],
    )
    .await?;

    Ok(())
}

pub async fn create_ncn_reward_router(
    handler: &CliHandler,
    operator: &Pubkey,
    ncn_fee_group: NcnFeeGroup,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let operator = *operator;

    let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    let ncn_reward_router_account = get_account(handler, &ncn_reward_router).await?;

    // Skip if ncn reward router already exists
    if ncn_reward_router_account.data.is_empty() {
        let initialize_ncn_reward_router_ix = InitializeNcnRewardRouterBuilder::new()
            .ncn(ncn)
            .operator(operator)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .payer(keypair.pubkey())
            .restaking_program(handler.restaking_program_id)
            .system_program(system_program::id())
            .ncn_fee_group(ncn_fee_group.group)
            .epoch(epoch)
            .instruction();

        send_and_log_transaction(
            client,
            keypair,
            &[initialize_ncn_reward_router_ix],
            &[],
            "Initialized NCN Reward Router",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("NCN Fee Group: {:?}", ncn_fee_group.group),
                format!("Epoch: {:?}", epoch),
            ],
        )
        .await?;
    }

    Ok(())
}

pub async fn route_base_rewards(handler: &CliHandler) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (epoch_snapshot, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (ballot_box, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (base_reward_router, _, _) =
        BaseRewardRouter::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (base_reward_receiver, _, _) =
        BaseRewardReceiver::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    // Using max iterations defined in BaseRewardRouter
    let max_iterations: u16 = BaseRewardRouter::MAX_ROUTE_BASE_ITERATIONS;

    let mut still_routing = true;
    while still_routing {
        let route_base_rewards_ix = RouteBaseRewardsBuilder::new()
            .ncn(ncn)
            .epoch_snapshot(epoch_snapshot)
            .ballot_box(ballot_box)
            .base_reward_router(base_reward_router)
            .base_reward_receiver(base_reward_receiver)
            .restaking_program(handler.restaking_program_id)
            .max_iterations(max_iterations)
            .epoch(epoch)
            .instruction();

        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            route_base_rewards_ix,
        ];

        send_and_log_transaction(
            client,
            keypair,
            &instructions,
            &[],
            "Routed Base Rewards",
            &[
                format!("NCN: {:?}", ncn),
                format!("Epoch: {:?}", epoch),
                format!("Max iterations: {:?}", max_iterations),
            ],
        )
        .await?;

        // Check if we need to continue routing
        let base_reward_router_account = get_base_reward_router(handler).await?;
        still_routing = base_reward_router_account.still_routing();
    }

    Ok(())
}

pub async fn route_ncn_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    ncn_fee_group: NcnFeeGroup,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let operator = *operator;

    let (operator_snapshot, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        &operator,
        &ncn,
        epoch,
    );

    let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    // Using max iterations defined in NcnRewardRouter
    let max_iterations: u16 = NcnRewardRouter::MAX_ROUTE_NCN_ITERATIONS;

    let mut still_routing = true;
    while still_routing {
        let route_ncn_rewards_ix = RouteNcnRewardsBuilder::new()
            .ncn(ncn)
            .operator(operator)
            .operator_snapshot(operator_snapshot)
            .ncn_reward_router(ncn_reward_router)
            .ncn_reward_receiver(ncn_reward_receiver)
            .restaking_program(handler.restaking_program_id)
            .ncn_fee_group(ncn_fee_group.group)
            .max_iterations(max_iterations)
            .epoch(epoch)
            .instruction();

        let instructions = vec![
            ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
            route_ncn_rewards_ix,
        ];

        send_and_log_transaction(
            client,
            keypair,
            &instructions,
            &[],
            "Routed NCN Rewards",
            &[
                format!("NCN: {:?}", ncn),
                format!("Operator: {:?}", operator),
                format!("NCN Fee Group: {:?}", ncn_fee_group.group),
                format!("Epoch: {:?}", epoch),
                format!("Max iterations: {:?}", max_iterations),
            ],
        )
        .await?;

        // Check if we need to continue routing
        let ncn_reward_router_account =
            get_ncn_reward_router(handler, ncn_fee_group, &operator).await?;
        still_routing = ncn_reward_router_account.still_routing();
    }

    Ok(())
}

pub async fn distribute_base_ncn_rewards(
    handler: &CliHandler,
    operator: &Pubkey,
    ncn_fee_group: NcnFeeGroup,
) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;
    let operator = *operator;

    let (ncn_config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (base_reward_router, _, _) =
        BaseRewardRouter::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (base_reward_receiver, _, _) =
        BaseRewardReceiver::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let (ncn_reward_router, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    let (ncn_reward_receiver, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        &operator,
        &ncn,
        epoch,
    );

    let distribute_base_ncn_rewards_ix = DistributeBaseNcnRewardRouteBuilder::new()
        .config(ncn_config)
        .ncn(ncn)
        .operator(operator)
        .base_reward_router(base_reward_router)
        .base_reward_receiver(base_reward_receiver)
        .ncn_reward_router(ncn_reward_router)
        .ncn_reward_receiver(ncn_reward_receiver)
        .restaking_program(handler.restaking_program_id)
        .system_program(system_program::id())
        .ncn_fee_group(ncn_fee_group.group)
        .epoch(epoch)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[distribute_base_ncn_rewards_ix],
        &[],
        "Distributed Base NCN Rewards",
        &[
            format!("NCN: {:?}", ncn),
            format!("Operator: {:?}", operator),
            format!("NCN Fee Group: {:?}", ncn_fee_group.group),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

//TODO distribute base rewards
//TODO distribute ncn vault rewards
//TODO distribute ncn operator rewards

pub async fn admin_set_tie_breaker(handler: &CliHandler, meta_merkle_root: [u8; 32]) -> Result<()> {
    let keypair = handler.keypair()?;
    let client = handler.rpc_client();

    let ncn = *handler.ncn()?;
    let epoch = handler.epoch;

    let (ncn_config, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);

    let (ballot_box, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, &ncn, epoch);

    let set_tie_breaker_ix = AdminSetTieBreakerBuilder::new()
        .config(ncn_config)
        .ballot_box(ballot_box)
        .ncn(ncn)
        .tie_breaker_admin(keypair.pubkey())
        .meta_merkle_root(meta_merkle_root)
        .epoch(epoch)
        .restaking_program(handler.restaking_program_id)
        .instruction();

    send_and_log_transaction(
        client,
        keypair,
        &[set_tie_breaker_ix],
        &[],
        "Set Tie Breaker",
        &[
            format!("NCN: {:?}", ncn),
            format!("Meta Merkle Root: {:?}", meta_merkle_root),
            format!("Epoch: {:?}", epoch),
        ],
    )
    .await?;

    Ok(())
}

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
        client,
        keypair,
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
        client,
        keypair,
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
        client,
        keypair,
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
        client,
        keypair,
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
        client,
        keypair,
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
        client,
        keypair,
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
        client,
        keypair,
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
            client,
            keypair,
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
            client,
            keypair,
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

    // let config = RpcSendTransactionConfig {
    //     skip_preflight: true,
    //     ..RpcSendTransactionConfig::default()
    // };
    // let result = client
    //     .send_and_confirm_transaction_with_spinner_and_config(
    //         &tx,
    //         CommitmentConfig::confirmed(),
    //         config,
    //     )
    //     .await;

    let result = client.send_and_confirm_transaction(&tx).await;

    if let Err(e) = result {
        return Err(anyhow!("\nError: \n\n{:?}\n\n", e));
    }

    Ok(result.unwrap())
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

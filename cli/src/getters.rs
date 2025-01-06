use std::mem::size_of;

use crate::handler::CliHandler;
use anyhow::{Ok, Result};
use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as TipRouterConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
use jito_vault_core::vault::Vault;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{account::Account, pubkey::Pubkey};

// ---------------------- HELPERS ----------------------
// So we can switch between the two implementations
pub async fn get_account(handler: &CliHandler, account: &Pubkey) -> Result<Account> {
    let client = handler.rpc_client();
    let account = client.get_account(account).await?;
    Ok(account)
}

// ---------------------- TIP ROUTER ----------------------
pub async fn get_tip_router_config(handler: &CliHandler) -> Result<TipRouterConfig> {
    let (address, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;
    let account = TipRouterConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_registry(handler: &CliHandler) -> Result<VaultRegistry> {
    let (address, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;
    let account = VaultRegistry::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_weight_table(handler: &CliHandler) -> Result<WeightTable> {
    let (address, _, _) = WeightTable::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = WeightTable::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_snapshot(handler: &CliHandler) -> Result<EpochSnapshot> {
    let (address, _, _) = EpochSnapshot::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = EpochSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
) -> Result<OperatorSnapshot> {
    let (address, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = OperatorSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ballot_box(handler: &CliHandler) -> Result<BallotBox> {
    let (address, _, _) = BallotBox::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = BallotBox::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_router(handler: &CliHandler) -> Result<BaseRewardRouter> {
    let (address, _, _) = BaseRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = BaseRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_reciever(handler: &CliHandler) -> Result<Account> {
    let (address, _, _) = BaseRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    Ok(account)
}

pub async fn get_ncn_reward_router(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
) -> Result<NcnRewardRouter> {
    let (address, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = NcnRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_reward_reciever(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
) -> Result<Account> {
    let (address, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    Ok(account)
}

// ---------------------- RESTAKING ----------------------

pub async fn get_restaking_config(handler: &CliHandler) -> Result<RestakingConfig> {
    let (address, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);
    let account = get_account(handler, &address).await?;
    let account = RestakingConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn(handler: &CliHandler) -> Result<Ncn> {
    let account = get_account(handler, handler.ncn()?).await?;
    let account = Ncn::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault(handler: &CliHandler, vault: &Pubkey) -> Result<Vault> {
    let account = get_account(handler, vault).await?;
    let account = Vault::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator(handler: &CliHandler, operator: &Pubkey) -> Result<Operator> {
    let account = get_account(handler, operator).await?;
    let account = Operator::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_operator_state(
    handler: &CliHandler,
    operator: &Pubkey,
) -> Result<NcnOperatorState> {
    let (address, _, _) = NcnOperatorState::find_program_address(
        &handler.restaking_program_id,
        handler.ncn()?,
        operator,
    );

    let account = get_account(handler, &address).await?;
    let account = NcnOperatorState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_all_operators_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();

    let ncn_operator_state_size = size_of::<NcnOperatorState>() + 8;

    let size_filter = RpcFilterType::DataSize(ncn_operator_state_size as u64);

    let ncn_filter = RpcFilterType::Memcmp(Memcmp::new(
        8,                                                           // offset
        MemcmpEncodedBytes::Bytes(handler.ncn()?.to_bytes().into()), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, ncn_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: ncn_operator_state_size,
            }),
            commitment: Some(handler.commitment),
            min_context_slot: None,
        },
        with_context: Some(false),
    };

    let results = client
        .get_program_accounts_with_config(&handler.restaking_program_id, config)
        .await?;

    let accounts: Vec<(Pubkey, NcnOperatorState)> = results
        .iter()
        .filter_map(|result| {
            NcnOperatorState::try_from_slice_unchecked(result.1.data.as_slice())
                .map(|account| (result.0, *account))
                .ok()
        })
        .collect();

    let operators = accounts
        .iter()
        .map(|(_, ncn_operator_state)| ncn_operator_state.operator.clone())
        .collect::<Vec<Pubkey>>();

    Ok(operators)
}

pub async fn get_all_vaults_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();

    let ncn_vault_ticket_size = size_of::<NcnVaultTicket>() + 8;

    let size_filter = RpcFilterType::DataSize(ncn_vault_ticket_size as u64);

    let ncn_filter = RpcFilterType::Memcmp(Memcmp::new(
        8,                                                           // offset
        MemcmpEncodedBytes::Bytes(handler.ncn()?.to_bytes().into()), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, ncn_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: ncn_vault_ticket_size,
            }),
            commitment: Some(handler.commitment),
            min_context_slot: None,
        },
        with_context: Some(false),
    };

    let results = client
        .get_program_accounts_with_config(&handler.restaking_program_id, config)
        .await?;

    let accounts: Vec<(Pubkey, NcnVaultTicket)> = results
        .iter()
        .filter_map(|result| {
            NcnVaultTicket::try_from_slice_unchecked(result.1.data.as_slice())
                .map(|account| (result.0, *account))
                .ok()
        })
        .collect();

    let vaults = accounts
        .iter()
        .map(|(_, ncn_operator_state)| ncn_operator_state.vault.clone())
        .collect::<Vec<Pubkey>>();

    Ok(vaults)
}

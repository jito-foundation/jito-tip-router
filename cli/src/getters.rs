use std::fmt;
use std::mem::size_of;

use crate::handler::CliHandler;
use anyhow::Result;
use borsh::BorshDeserialize;
use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
    operator_vault_ticket::OperatorVaultTicket,
};
use jito_tip_router_core::{
    account_payer::AccountPayer,
    ballot_box::BallotBox,
    base_fee_group::BaseFeeGroup,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as TipRouterConfig,
    constants::JITOSOL_POOL_ADDRESS,
    epoch_marker::EpochMarker,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
use jito_vault_core::{
    vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{account::Account, pubkey::Pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_stake_pool::{find_withdraw_authority_program_address, state::StakePool};

// ---------------------- HELPERS ----------------------
// So we can switch between the two implementations
pub async fn get_account(handler: &CliHandler, account: &Pubkey) -> Result<Option<Account>> {
    let client = handler.rpc_client();
    let account = client
        .get_account_with_commitment(account, handler.commitment)
        .await?;

    Ok(account.value)
}

pub async fn get_current_epoch(handler: &CliHandler) -> Result<u64> {
    let client = handler.rpc_client();
    let epoch = client.get_epoch_info().await?.epoch;
    Ok(epoch)
}

pub async fn get_current_slot(handler: &CliHandler) -> Result<u64> {
    let client = handler.rpc_client();
    let slot = client.get_slot().await?;
    Ok(slot)
}

pub async fn get_current_epoch_and_slot(handler: &CliHandler) -> Result<(u64, u64)> {
    let epoch = get_current_epoch(handler).await?;
    let slot = get_current_slot(handler).await?;
    Ok((epoch, slot))
}

pub async fn get_current_epoch_and_slot_unsafe(handler: &CliHandler) -> (u64, u64) {
    get_current_epoch_and_slot(handler)
        .await
        .expect("Failed to get epoch and slot")
}

// ---------------------- TIP ROUTER ----------------------
pub async fn get_tip_router_config(handler: &CliHandler) -> Result<TipRouterConfig> {
    let (address, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = TipRouterConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_registry(handler: &CliHandler) -> Result<VaultRegistry> {
    let (address, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("VR Account not found"));
    }
    let account = account.unwrap();

    let account = VaultRegistry::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_state(handler: &CliHandler, epoch: u64) -> Result<EpochState> {
    let (address, _, _) =
        EpochState::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = EpochState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_weight_table(handler: &CliHandler, epoch: u64) -> Result<WeightTable> {
    let (address, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = WeightTable::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_snapshot(handler: &CliHandler, epoch: u64) -> Result<EpochSnapshot> {
    let (address, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = EpochSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<OperatorSnapshot> {
    let (address, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = OperatorSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ballot_box(handler: &CliHandler, epoch: u64) -> Result<BallotBox> {
    let (address, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = BallotBox::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_router(handler: &CliHandler, epoch: u64) -> Result<BaseRewardRouter> {
    let (address, _, _) = BaseRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = BaseRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_receiver(
    handler: &CliHandler,
    epoch: u64,
) -> Result<(Pubkey, Account)> {
    let (address, _, _) = BaseRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok((address, account))
}

pub async fn get_ncn_reward_router(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
    epoch: u64,
) -> Result<NcnRewardRouter> {
    let (address, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_reward_receiver(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
    epoch: u64,
) -> Result<(Pubkey, Account)> {
    let (address, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok((address, account))
}

pub async fn get_receiver_rewards(handler: &CliHandler, address: &Pubkey) -> Result<u64> {
    let account = get_account(handler, address).await?;

    let rent = handler
        .rpc_client()
        .get_minimum_balance_for_rent_exemption(0)
        .await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok(account.lamports - rent)
}

pub async fn get_base_reward_receiver_rewards(handler: &CliHandler, epoch: u64) -> Result<u64> {
    let (address, _) = get_base_reward_receiver(handler, epoch).await?;
    get_receiver_rewards(handler, &address).await
}

pub async fn get_ncn_reward_receiver_rewards(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
    epoch: u64,
) -> Result<u64> {
    let (address, _) = get_ncn_reward_receiver(handler, ncn_fee_group, operator, epoch).await?;
    get_receiver_rewards(handler, &address).await
}

#[allow(clippy::large_stack_frames)]
pub async fn get_total_rewards_to_be_distributed(handler: &CliHandler, epoch: u64) -> Result<u64> {
    let all_operators = {
        let ballot_box = get_ballot_box(handler, epoch).await?;
        let winning_ballot = ballot_box.get_winning_ballot_tally()?;
        let winning_ballot_index = winning_ballot.index();

        ballot_box
            .operator_votes()
            .iter()
            .filter_map(|vote| {
                if vote.ballot_index() == winning_ballot_index {
                    Some(*vote.operator())
                } else {
                    None
                }
            })
            .collect::<Vec<Pubkey>>()
    };

    let all_ncn_groups = {
        let epoch_snapshot = get_epoch_snapshot(handler, epoch).await?;
        let fees = *epoch_snapshot.fees();
        NcnFeeGroup::all_groups()
            .iter()
            .filter_map(|group| {
                if fees.ncn_fee_bps(*group).unwrap() > 0 {
                    Some(*group)
                } else {
                    None
                }
            })
            .collect::<Vec<NcnFeeGroup>>()
    };

    let mut total_amount_to_distribute = 0;
    {
        let result = get_base_reward_receiver_rewards(handler, epoch).await;
        if result.is_err() {
            return Ok(0);
        }

        total_amount_to_distribute += result.unwrap();
    }

    for operator in all_operators.iter() {
        for group in all_ncn_groups.iter() {
            let result = get_ncn_reward_receiver_rewards(handler, *group, operator, epoch).await;

            if result.is_err() {
                continue;
            }

            total_amount_to_distribute += result.unwrap();
        }
    }

    Ok(total_amount_to_distribute)
}

pub async fn get_account_payer(handler: &CliHandler) -> Result<Account> {
    let (address, _, _) =
        AccountPayer::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok(account)
}

pub async fn get_epoch_marker(handler: &CliHandler, epoch: u64) -> Result<EpochMarker> {
    let (address, _, _) =
        EpochMarker::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = EpochMarker::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_is_epoch_completed(handler: &CliHandler, epoch: u64) -> Result<bool> {
    let (address, _, _) =
        EpochMarker::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    Ok(account.is_some())
}

// ---------------------- RESTAKING ----------------------

pub async fn get_restaking_config(handler: &CliHandler) -> Result<RestakingConfig> {
    let (address, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);
    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = RestakingConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn(handler: &CliHandler) -> Result<Ncn> {
    let account = get_account(handler, handler.ncn()?).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = Ncn::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault(handler: &CliHandler, vault: &Pubkey) -> Result<Vault> {
    let account = get_account(handler, vault)
        .await?
        .expect("Account not found");
    let account = Vault::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator(handler: &CliHandler, operator: &Pubkey) -> Result<Operator> {
    let account = get_account(handler, operator).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

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

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnOperatorState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_ncn_ticket(handler: &CliHandler, vault: &Pubkey) -> Result<VaultNcnTicket> {
    let (address, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, vault, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = VaultNcnTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_vault_ticket(handler: &CliHandler, vault: &Pubkey) -> Result<NcnVaultTicket> {
    let (address, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, handler.ncn()?, vault);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnVaultTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_operator_delegation(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<VaultOperatorDelegation> {
    let (address, _, _) =
        VaultOperatorDelegation::find_program_address(&handler.vault_program_id, vault, operator);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = VaultOperatorDelegation::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_vault_ticket(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<OperatorVaultTicket> {
    let (address, _, _) =
        OperatorVaultTicket::find_program_address(&handler.restaking_program_id, operator, vault);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = OperatorVaultTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_stake_pool(handler: &CliHandler) -> Result<StakePool> {
    let stake_pool = JITOSOL_POOL_ADDRESS;
    let account = get_account(handler, &stake_pool).await?.unwrap();
    let mut data_slice = account.data.as_slice();
    let account = StakePool::deserialize(&mut data_slice)
        .map_err(|_| anyhow::anyhow!("Invalid stake pool account"))?;

    Ok(account)
}

pub struct StakePoolAccounts {
    pub stake_pool_program_id: Pubkey,
    pub stake_pool_address: Pubkey,
    pub stake_pool: StakePool,
    pub stake_pool_withdraw_authority: Pubkey,
    pub referrer_pool_tokens_account: Pubkey,
}

pub async fn get_stake_pool_accounts(handler: &CliHandler) -> Result<StakePoolAccounts> {
    let stake_pool_program_id = spl_stake_pool::id();
    let stake_pool_address = JITOSOL_POOL_ADDRESS;
    let stake_pool = get_stake_pool(handler).await?;

    let (stake_pool_withdraw_authority, _) =
        find_withdraw_authority_program_address(&spl_stake_pool::id(), &stake_pool_address);

    let referrer_pool_tokens_account = {
        let tip_router_config = get_tip_router_config(handler).await?;
        let base_fee_wallet = tip_router_config
            .fee_config
            .base_fee_wallet(BaseFeeGroup::default())?;
        get_associated_token_address(base_fee_wallet, &stake_pool.pool_mint)
    };

    let accounts = StakePoolAccounts {
        stake_pool_program_id,
        stake_pool_address,
        stake_pool,
        stake_pool_withdraw_authority,
        referrer_pool_tokens_account,
    };

    Ok(accounts)
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
        .map(|(_, ncn_operator_state)| ncn_operator_state.operator)
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
        .map(|(_, ncn_operator_state)| ncn_operator_state.vault)
        .collect::<Vec<Pubkey>>();

    Ok(vaults)
}

pub async fn get_total_epoch_rent_cost(handler: &CliHandler) -> Result<u64> {
    let current_epoch = handler.epoch;
    let client = handler.rpc_client();

    let operator_count = {
        let all_operators = get_all_operators_in_ncn(handler).await?;
        all_operators.len() as u64
    };

    let fee_group_count = {
        let config = get_tip_router_config(handler).await?;
        let current_fees = config.fee_config.current_fees(current_epoch);
        let mut fee_group_count = 0;
        for group in NcnFeeGroup::all_groups() {
            let fee = current_fees.ncn_fee_bps(group)?;
            if fee > 0 {
                fee_group_count += 1;
            }
        }
        fee_group_count as u64
    };

    let mut rent_cost = 0;

    rent_cost += client
        .get_minimum_balance_for_rent_exemption(EpochState::SIZE)
        .await?;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(WeightTable::SIZE)
        .await?;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(EpochSnapshot::SIZE)
        .await?;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(OperatorSnapshot::SIZE)
        .await?
        * operator_count;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(BallotBox::SIZE)
        .await?;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(BaseRewardRouter::SIZE)
        .await?;
    // Base Reward Receiver
    rent_cost += client.get_minimum_balance_for_rent_exemption(0).await?;
    rent_cost += client
        .get_minimum_balance_for_rent_exemption(NcnRewardRouter::SIZE)
        .await?
        * operator_count
        * fee_group_count;
    rent_cost +=
        client.get_minimum_balance_for_rent_exemption(0).await? * operator_count * fee_group_count;

    Ok(rent_cost)
}

pub async fn get_all_tickets(handler: &CliHandler) -> Result<Vec<NcnTickets>> {
    let client = handler.rpc_client();

    let all_vaults = get_all_vaults_in_ncn(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;

    let restaking_config = get_restaking_config(handler).await?;

    let slot = client.get_epoch_info().await?.absolute_slot;
    let epoch_length = restaking_config.epoch_length();

    let mut tickets = Vec::new();
    for operator in all_operators.iter() {
        for vault in all_vaults.iter() {
            tickets.push(NcnTickets::fetch(handler, operator, vault, slot, epoch_length).await?);
        }
    }

    Ok(tickets)
}

pub struct NcnTickets {
    pub slot: u64,
    pub epoch_length: u64,
    pub ncn: Pubkey,
    pub vault: Pubkey,
    pub vault_account: Vault,
    pub operator: Pubkey,
    pub ncn_vault_ticket_address: Pubkey,
    pub ncn_vault_ticket: Option<NcnVaultTicket>,
    pub vault_ncn_ticket_address: Pubkey,
    pub vault_ncn_ticket: Option<VaultNcnTicket>,
    pub vault_operator_delegation_address: Pubkey,
    pub vault_operator_delegation: Option<VaultOperatorDelegation>,
    pub operator_vault_ticket_address: Pubkey,
    pub operator_vault_ticket: Option<OperatorVaultTicket>,
    pub ncn_operator_state_address: Pubkey,
    pub ncn_operator_state: Option<NcnOperatorState>,
}

impl NcnTickets {
    const DNE: u8 = 0;
    const NOT_ACTIVE: u8 = 1;
    const ACTIVE: u8 = 2;

    pub async fn fetch(
        handler: &CliHandler,
        operator: &Pubkey,
        vault: &Pubkey,
        slot: u64,
        epoch_length: u64,
    ) -> Result<Self> {
        let ncn = handler.ncn().expect("NCN not found");

        let (ncn_vault_ticket_address, _, _) =
            NcnVaultTicket::find_program_address(&handler.restaking_program_id, ncn, vault);
        let ncn_vault_ticket = get_ncn_vault_ticket(handler, vault).await;
        let ncn_vault_ticket = {
            match ncn_vault_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        };

        let (vault_ncn_ticket_address, _, _) =
            VaultNcnTicket::find_program_address(&handler.vault_program_id, vault, ncn);
        let vault_ncn_ticket = get_vault_ncn_ticket(handler, vault).await;
        let vault_ncn_ticket = {
            match vault_ncn_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        };

        let (vault_operator_delegation_address, _, _) =
            VaultOperatorDelegation::find_program_address(
                &handler.vault_program_id,
                vault,
                operator,
            );
        let vault_operator_delegation =
            get_vault_operator_delegation(handler, vault, operator).await;
        let vault_operator_delegation = {
            match vault_operator_delegation {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        };

        let (operator_vault_ticket_address, _, _) = OperatorVaultTicket::find_program_address(
            &handler.restaking_program_id,
            operator,
            vault,
        );
        let operator_vault_ticket = get_operator_vault_ticket(handler, vault, operator).await;
        let operator_vault_ticket = {
            match operator_vault_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        };

        let (ncn_operator_state_address, _, _) =
            NcnOperatorState::find_program_address(&handler.restaking_program_id, ncn, operator);
        let ncn_operator_state = get_ncn_operator_state(handler, operator).await;
        let ncn_operator_state = {
            match ncn_operator_state {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        return Err(e);
                    }
                }
            }
        };

        let vault_account = get_vault(handler, vault).await.expect("Vault not found");

        Ok(Self {
            slot,
            epoch_length,
            ncn: *ncn,
            vault: *vault,
            vault_account,
            operator: *operator,
            ncn_vault_ticket,
            vault_ncn_ticket,
            vault_operator_delegation,
            operator_vault_ticket,
            ncn_operator_state,
            ncn_vault_ticket_address,
            vault_ncn_ticket_address,
            vault_operator_delegation_address,
            operator_vault_ticket_address,
            ncn_operator_state_address,
        })
    }

    pub fn ncn_operator(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .ncn_opt_in_state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn operator_ncn(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .operator_opt_in_state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn ncn_vault(&self) -> u8 {
        if self.ncn_vault_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn vault_ncn(&self) -> u8 {
        if self.vault_ncn_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .vault_ncn_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn operator_vault(&self) -> u8 {
        if self.operator_vault_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .operator_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn vault_operator(&self) -> u8 {
        if self.vault_operator_delegation.is_none() {
            return Self::DNE;
        }

        if self
            .vault_operator_delegation
            .as_ref()
            .unwrap()
            .delegation_state
            .total_security()
            .unwrap()
            > 0
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }
}

impl fmt::Display for NcnTickets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper closure for checkmarks in summary
        let check = |state: u8| -> &str {
            match state {
                Self::DNE => "❌",
                Self::NOT_ACTIVE => "🕘",
                Self::ACTIVE => "✅",
                _ => "",
            }
        };

        writeln!(f, "\n")?;
        writeln!(f, "------------------ STATE ---------------------\n")?;
        writeln!(f, "NCN:      {}", self.ncn)?;
        writeln!(f, "Operator: {}", self.operator)?;
        writeln!(f, "Vault:    {}", self.vault)?;
        writeln!(f, "\n")?;
        writeln!(
            f,
            "NCN      -> Operator: {} {}",
            check(self.ncn_operator()),
            self.ncn_operator_state_address
        )?;
        writeln!(
            f,
            "Operator -> NCN:      {} {}",
            check(self.operator_ncn()),
            self.ncn_operator_state_address
        )?;
        writeln!(
            f,
            "NCN      -> Vault:    {} {}",
            check(self.ncn_vault()),
            self.ncn_vault_ticket_address
        )?;
        writeln!(
            f,
            "Vault    -> NCN:      {} {}",
            check(self.vault_ncn()),
            self.vault_ncn_ticket_address
        )?;
        writeln!(
            f,
            "Operator -> Vault:    {} {}",
            check(self.operator_vault()),
            self.operator_vault_ticket_address
        )?;

        let st_mint = self.vault_account.supported_mint;
        let delegation = {
            if self.vault_operator_delegation.is_some() {
                self.vault_operator_delegation
                    .unwrap()
                    .delegation_state
                    .total_security()
                    .unwrap()
            } else {
                0
            }
        };

        writeln!(
            f,
            "Vault    -> Operator: {} {} {}: {}",
            check(self.vault_operator()),
            self.vault_operator_delegation_address,
            st_mint,
            delegation
        )?;
        writeln!(f, "\n")?;

        Ok(())
    }
}

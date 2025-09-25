use std::mem::size_of;
use std::str::FromStr;
use std::{fmt, time::Duration};

use super::spl_stake_pool::{find_withdraw_authority_program_address, StakePool};
use crate::handler::CliHandler;
use anyhow::Result;
use borsh::BorshDeserialize;
use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::slot_toggle::SlotToggleState;
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
    operator_vault_ticket::OperatorVaultTicket,
};
use jito_tip_distribution_sdk::TipDistributionAccount;
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
    config::Config as VaultConfig, vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
    vault_update_state_tracker::VaultUpdateStateTracker,
};
use log::info;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::RpcGetVoteAccountsConfig;
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_commitment_config::CommitmentConfig;
use solana_sdk::clock::DEFAULT_SLOTS_PER_EPOCH;
use solana_sdk::{account::Account, pubkey::Pubkey};
use spl_associated_token_account_interface::address::get_associated_token_address;
use tokio::time::sleep;

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

pub async fn get_guaranteed_epoch_and_slot(handler: &CliHandler) -> (u64, u64) {
    loop {
        let current_epoch_and_slot_result = get_current_epoch_and_slot(handler).await;

        if let Ok(result) = current_epoch_and_slot_result {
            return result;
        }

        info!("Could not fetch current epoch and slot. Retrying...");
        sleep(Duration::from_secs(1)).await;
    }
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

pub async fn get_vault_config(handler: &CliHandler) -> Result<VaultConfig> {
    let (address, _, _) = VaultConfig::find_program_address(&handler.vault_program_id);
    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Vault Config account not found"));
    }
    let account = account.unwrap();

    let account = VaultConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_restaking_config(handler: &CliHandler) -> Result<RestakingConfig> {
    let (address, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);
    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Restaking Config Account not found"));
    }
    let account = account.unwrap();

    let account = RestakingConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn(handler: &CliHandler) -> Result<Ncn> {
    let account = get_account(handler, handler.ncn()?).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("NCN account not found"));
    }
    let account = account.unwrap();

    let account = Ncn::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault(handler: &CliHandler, vault: &Pubkey) -> Result<Vault> {
    let account = get_account(handler, vault).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Vault account not found"));
    }
    let account = account.unwrap();

    let account = Vault::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_update_state_tracker(
    handler: &CliHandler,
    vault: &Pubkey,
    ncn_epoch: u64,
) -> Result<VaultUpdateStateTracker> {
    let (vault_update_state_tracker, _, _) =
        VaultUpdateStateTracker::find_program_address(&handler.vault_program_id, vault, ncn_epoch);

    let account = get_account(handler, &vault_update_state_tracker).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!(
            "Vault Update State Tracker account not found"
        ));
    }
    let account = account.unwrap();

    let account = VaultUpdateStateTracker::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator(handler: &CliHandler, operator: &Pubkey) -> Result<Operator> {
    let account = get_account(handler, operator).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Operator account not found"));
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
        return Err(anyhow::anyhow!("NCN Operator State account not found"));
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
        return Err(anyhow::anyhow!("Vault NCN Ticket account not found"));
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
        return Err(anyhow::anyhow!("NCN Vault Ticket account not found"));
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
        return Err(anyhow::anyhow!(
            "Vault Operator Delegation account not found"
        ));
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
        return Err(anyhow::anyhow!("Operator Vault Ticket account not found"));
    }
    let account = account.unwrap();

    let account = OperatorVaultTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_stake_pool(handler: &CliHandler) -> Result<StakePool> {
    let stake_pool = JITOSOL_POOL_ADDRESS;
    let account = get_account(handler, &stake_pool).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Stake Pool account not found"));
    }
    let account = account.unwrap();

    let mut data_slice = account.data.as_slice();
    let account = StakePool::deserialize(&mut data_slice)
        .map_err(|_| anyhow::anyhow!("Invalid stake pool account"))?;

    Ok(account)
}

pub struct OptedInValidatorInfo {
    pub vote: Pubkey,
    pub identity: Pubkey,
    pub stake: u64,
    pub active: bool,
}

pub async fn get_all_opted_in_validators(
    handler: &CliHandler,
) -> Result<Vec<OptedInValidatorInfo>> {
    // Vote Account, Validator Identity, Validator Stake, active or not
    let client = handler.rpc_client();
    let client = RpcClient::new_with_timeout_and_commitment(
        client.url(),
        Duration::from_secs(3600),
        CommitmentConfig::processed(),
    );

    let tip_distribution_size = size_of::<TipDistributionAccount>() + 8;

    let size_filter = RpcFilterType::DataSize(tip_distribution_size as u64);

    let discriminator_filter = RpcFilterType::Memcmp(Memcmp::new(
        0,                                                                       // offset
        MemcmpEncodedBytes::Bytes([85, 64, 113, 198, 234, 94, 120, 123].into()), // encoded bytes
    ));
    let upload_auth_filter = RpcFilterType::Memcmp(Memcmp::new(
        8 + 32, // offset
        MemcmpEncodedBytes::Bytes(
            Pubkey::from_str("8F4jGUmxF36vQ6yabnsxX6AQVXdKBhs8kGSUuRKSg8Xt")
                .unwrap()
                .to_bytes()
                .into(),
        ), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, upload_auth_filter, discriminator_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 8,
                length: 32,
            }),
            commitment: Some(CommitmentConfig::processed()),
            min_context_slot: None,
        },
        with_context: Some(false),
        sort_results: None,
    };

    let results = client
        .get_program_accounts_with_config(&handler.tip_distribution_program_id, config)
        .await?;

    let vote_accounts: Vec<Pubkey> = results
        .iter()
        .map(|result| {
            let data_slice = result.1.data.as_slice();
            // Convert the slice to a fixed-size array
            Pubkey::new_from_array(data_slice[..32].try_into().expect("Slice must be 32 bytes"))
        })
        .collect();

    let mut validator_infos: Vec<OptedInValidatorInfo> = vec![];

    for vote_account in vote_accounts {
        let config = RpcGetVoteAccountsConfig {
            vote_pubkey: Some(vote_account.to_string()),
            ..RpcGetVoteAccountsConfig::default()
        };
        let status_result = client.get_vote_accounts_with_config(config).await;

        if status_result.is_err() {
            continue;
        }

        let status = status_result.unwrap();

        if !status.current.is_empty() {
            let info = status.current[0].clone();
            let identity = Pubkey::from_str(&info.node_pubkey)?;
            validator_infos.push(OptedInValidatorInfo {
                vote: vote_account,
                identity,
                stake: info.activated_stake,
                active: true,
            });
        } else if !status.delinquent.is_empty() {
            let info = status.delinquent[0].clone();
            let identity = Pubkey::from_str(&info.node_pubkey)?;

            validator_infos.push(OptedInValidatorInfo {
                vote: vote_account,
                identity,
                stake: info.activated_stake,
                active: false,
            });
        }
    }

    validator_infos.sort_by(|a, b| a.identity.cmp(&b.identity));
    validator_infos.dedup_by(|a, b| a.identity.eq(&b.identity));

    Ok(validator_infos)
}

pub struct StakePoolAccounts {
    pub stake_pool_program_id: Pubkey,
    pub stake_pool_address: Pubkey,
    pub stake_pool: StakePool,
    pub stake_pool_withdraw_authority: Pubkey,
    pub referrer_pool_tokens_account: Pubkey,
}

pub async fn get_stake_pool_accounts(handler: &CliHandler) -> Result<StakePoolAccounts> {
    let stake_pool_program_id = jito_tip_router_program::spl_stake_pool_id();
    let stake_pool_address = JITOSOL_POOL_ADDRESS;
    let stake_pool = get_stake_pool(handler).await?;

    let (stake_pool_withdraw_authority, _) = find_withdraw_authority_program_address(
        &jito_tip_router_program::spl_stake_pool_id(),
        &stake_pool_address,
    );

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

pub async fn get_all_sorted_operators_for_vault(
    handler: &CliHandler,
    vault: &Pubkey,
) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();
    let config = handler.get_rpc_program_accounts_with_config::<VaultOperatorDelegation>(vault)?;

    let results = client
        .get_program_accounts_with_config(&handler.vault_program_id, config)
        .await?;

    let accounts: Vec<(Pubkey, VaultOperatorDelegation)> = results
        .iter()
        .filter_map(|result| {
            VaultOperatorDelegation::try_from_slice_unchecked(result.1.data.as_slice())
                .map(|account| (result.0, *account))
                .ok()
        })
        .collect();

    let mut index_and_operator = accounts
        .iter()
        .map(|(_, vault_operator_delegation)| {
            (
                vault_operator_delegation.index(),
                vault_operator_delegation.operator,
            )
        })
        .collect::<Vec<(u64, Pubkey)>>();

    index_and_operator.sort_by_cached_key(|(index, _pubkey)| *index);

    let sorted_operators = index_and_operator
        .iter()
        .map(|(_index, pubkey)| *pubkey)
        .collect::<Vec<Pubkey>>();

    Ok(sorted_operators)
}

pub async fn get_all_operators_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();
    let config =
        handler.get_rpc_program_accounts_with_config::<NcnOperatorState>(handler.ncn()?)?;

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

pub async fn get_all_active_operators_in_ncn(
    handler: &CliHandler,
    epoch: u64,
) -> Result<Vec<Pubkey>> {
    let active_slot = epoch * DEFAULT_SLOTS_PER_EPOCH + 1;
    let operators = get_all_operators_in_ncn(handler).await?;

    let mut active_operators = vec![];
    for operator in operators {
        let result = get_ncn_operator_state(handler, &operator).await;

        if result.is_err() {
            continue;
        }

        let ncn_operator_state = result.unwrap();
        let ncn_operator_state_toggle_state = ncn_operator_state
            .ncn_opt_in_state
            .state(active_slot, DEFAULT_SLOTS_PER_EPOCH)
            .unwrap();

        match ncn_operator_state_toggle_state {
            SlotToggleState::Active => active_operators.push(operator),
            _ => continue,
        };
    }

    Ok(active_operators)
}

pub async fn get_all_vaults(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();

    let vault_size = size_of::<Vault>() + 8;

    let size_filter = RpcFilterType::DataSize(vault_size as u64);

    let vault_filter = RpcFilterType::Memcmp(Memcmp::new(
        0,                                                        // offset
        MemcmpEncodedBytes::Bytes([Vault::DISCRIMINATOR].into()), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, vault_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: 0,
            }),
            commitment: Some(handler.commitment),
            min_context_slot: None,
        },
        with_context: Some(false),
        sort_results: None,
    };

    let results = client
        .get_program_accounts_with_config(&handler.vault_program_id, config)
        .await?;

    let vaults: Vec<Pubkey> = results.iter().map(|result| result.0).collect();

    Ok(vaults)
}

pub async fn get_all_vaults_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();
    let config = handler.get_rpc_program_accounts_with_config::<NcnVaultTicket>(handler.ncn()?)?;

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
    const STAKE: u8 = 10;
    const NO_STAKE: u8 = 11;
    // To allow for legacy state to exist in database
    const STATE_OFFSET: u8 = 100;
    const INACTIVE: u8 = Self::STATE_OFFSET;
    const WARM_UP: u8 = Self::STATE_OFFSET + 1;
    const ACTIVE: u8 = Self::STATE_OFFSET + 2;
    const COOLDOWN: u8 = Self::STATE_OFFSET + 3;

    pub async fn fetch(
        handler: &CliHandler,
        operator: &Pubkey,
        vault: &Pubkey,
        slot: u64,
        epoch_length: u64,
    ) -> Result<Self> {
        let ncn = handler.ncn()?;
        let vault_account = get_vault(handler, vault).await?;

        let (ncn_vault_ticket_address, _, _) =
            NcnVaultTicket::find_program_address(&handler.restaking_program_id, ncn, vault);
        let ncn_vault_ticket = get_ncn_vault_ticket(handler, vault).await;
        let ncn_vault_ticket = ncn_vault_ticket.ok();

        let (vault_ncn_ticket_address, _, _) =
            VaultNcnTicket::find_program_address(&handler.vault_program_id, vault, ncn);
        let vault_ncn_ticket = get_vault_ncn_ticket(handler, vault).await;
        let vault_ncn_ticket = vault_ncn_ticket.ok();

        let (vault_operator_delegation_address, _, _) =
            VaultOperatorDelegation::find_program_address(
                &handler.vault_program_id,
                vault,
                operator,
            );
        let vault_operator_delegation =
            get_vault_operator_delegation(handler, vault, operator).await;
        let vault_operator_delegation = vault_operator_delegation.ok();

        let (operator_vault_ticket_address, _, _) = OperatorVaultTicket::find_program_address(
            &handler.restaking_program_id,
            operator,
            vault,
        );
        let operator_vault_ticket = get_operator_vault_ticket(handler, vault, operator).await;
        let operator_vault_ticket = operator_vault_ticket.ok();

        let (ncn_operator_state_address, _, _) =
            NcnOperatorState::find_program_address(&handler.restaking_program_id, ncn, operator);
        let ncn_operator_state = get_ncn_operator_state(handler, operator).await;
        let ncn_operator_state = ncn_operator_state.ok();

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

    pub const fn st_mint(&self) -> Pubkey {
        self.vault_account.supported_mint
    }

    pub fn delegation(&self) -> (u64, u64, u64) {
        if self.vault_operator_delegation.is_none() {
            return (0, 0, 0);
        }

        let delegation_state = self
            .vault_operator_delegation
            .as_ref()
            .unwrap()
            .delegation_state;

        (
            delegation_state.staked_amount(),
            delegation_state.cooling_down_amount(),
            delegation_state.total_security().unwrap(),
        )
    }

    pub fn ncn_operator(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        let state = self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .ncn_opt_in_state
            .state(self.slot, self.epoch_length)
            .unwrap() as u8;

        state + Self::STATE_OFFSET
    }

    pub fn operator_ncn(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        let state = self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .operator_opt_in_state
            .state(self.slot, self.epoch_length)
            .unwrap() as u8;

        state + Self::STATE_OFFSET
    }

    pub fn ncn_vault(&self) -> u8 {
        if self.ncn_vault_ticket.is_none() {
            return Self::DNE;
        }

        let state = self
            .ncn_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .state(self.slot, self.epoch_length)
            .unwrap() as u8;

        state + Self::STATE_OFFSET
    }

    pub fn vault_ncn(&self) -> u8 {
        if self.vault_ncn_ticket.is_none() {
            return Self::DNE;
        }

        let state = self
            .vault_ncn_ticket
            .as_ref()
            .unwrap()
            .state
            .state(self.slot, self.epoch_length)
            .unwrap() as u8;

        state + Self::STATE_OFFSET
    }

    pub fn operator_vault(&self) -> u8 {
        if self.operator_vault_ticket.is_none() {
            return Self::DNE;
        }

        let state = self
            .operator_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .state(self.slot, self.epoch_length)
            .unwrap() as u8;

        state + Self::STATE_OFFSET
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
            return Self::STAKE;
        }

        Self::NO_STAKE
    }
}

impl fmt::Display for NcnTickets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper closure for checkmarks in summary
        let check = |state: u8| -> &str {
            match state {
                Self::DNE => "❌",
                Self::STAKE => "🔋",
                Self::NO_STAKE => "🪫",
                Self::INACTIVE => "💤",
                Self::WARM_UP => "🔥",
                Self::ACTIVE => "✅",
                Self::COOLDOWN => "🥶",
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
            "DNE[{}] INACTIVE[{}] WARM_UP[{}] ACTIVE[{}] COOLDOWN[{}] NO_STAKE[{}] STAKE[{}]",
            check(Self::DNE),
            check(Self::INACTIVE),
            check(Self::WARM_UP),
            check(Self::ACTIVE),
            check(Self::COOLDOWN),
            check(Self::NO_STAKE),
            check(Self::STAKE),
        )?;
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

        writeln!(
            f,
            "Vault    -> Operator: {} {} {}: {}",
            check(self.vault_operator()),
            self.vault_operator_delegation_address,
            self.st_mint(),
            self.delegation().2
        )?;
        writeln!(f, "\n")?;

        Ok(())
    }
}

pub async fn get_tip_distribution_accounts_to_migrate(
    handler: &CliHandler,
    tip_distribution_program_id: &Pubkey,
    old_merkle_root_upload_authority: &Pubkey,
    epoch: u64,
) -> Result<Vec<Pubkey>> {
    let rpc_client = handler.rpc_client();
    let rpc_client = RpcClient::new_with_timeout(rpc_client.url(), Duration::from_secs(3600));

    // Filters assume merkle root is None
    let filters = vec![
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8     // Discriminator
                + 32, // Pubkey - validator_vote_account
            old_merkle_root_upload_authority.to_bytes().to_vec(),
        )),
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8    // Discriminator
                + 32 // Pubkey - validator_vote_account
                + 32 // Pubkey - merkle_root_upload_authority
                + 1, // Option - "None" merkle_root
            epoch.to_le_bytes().to_vec(),
        )),
    ];

    let tip_distribution_accounts = rpc_client
        .get_program_accounts_with_config(
            tip_distribution_program_id,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: Some(UiDataSliceConfig {
                        offset: 0,
                        length: 0,
                    }),
                    commitment: Some(handler.commitment),
                    min_context_slot: None,
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await?;

    Ok(tip_distribution_accounts
        .into_iter()
        .map(|(pubkey, _)| pubkey)
        .collect())
}

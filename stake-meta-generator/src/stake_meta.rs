//! Stake-meta calculation ported from jito-solana's tip-router snapshot service.
//!
//! The module owns the bank-derived calculation; the binary is responsible only
//! for loading a snapshot and persisting the resulting artifact.

use std::{collections::HashMap, mem::size_of, sync::Arc};

use borsh::de::BorshDeserialize;
use jito_priority_fee_distribution_sdk::{
    derive_priority_fee_distribution_account_address, PriorityFeeDistributionAccount,
};
use jito_stake_meta_types::{
    Delegation, PriorityFeeDistributionMeta, StakeMeta, StakeMetaCollection, TipDistributionMeta,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, TipDistributionAccount};
use jito_tip_payment_sdk::{
    Config, CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2,
    TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6,
    TIP_ACCOUNT_SEED_7,
};
use log::warn;
use solana_accounts_db::accounts_index::IndexKey;
use solana_runtime::{bank::Bank, stakes::StakeAccount};
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    pubkey::Pubkey,
};
use solana_stake_interface::{stake_history::StakeHistory, sysvar::stake_history};
use thiserror::Error;

const TIP_ACCOUNT_SEEDS: [&[u8]; 8] = [
    TIP_ACCOUNT_SEED_0,
    TIP_ACCOUNT_SEED_1,
    TIP_ACCOUNT_SEED_2,
    TIP_ACCOUNT_SEED_3,
    TIP_ACCOUNT_SEED_4,
    TIP_ACCOUNT_SEED_5,
    TIP_ACCOUNT_SEED_6,
    TIP_ACCOUNT_SEED_7,
];

#[derive(Debug, Error)]
pub enum StakeMetaError {
    #[error("stake metadata requires a frozen bank at slot {0}")]
    NotFrozen(u64),
    #[error("failed to read tip payment configuration: {0}")]
    AnchorError(String),
    #[error("overflow while calculating stake metadata")]
    CheckedMathError,
    #[error("no vote accounts found at slot {0} in epoch {1}")]
    NoVoteAccounts(u64, u64),
    #[error("failed to scan stake accounts: {0}")]
    ScanError(String),
}

/// Generate the stake-meta artifact content for a frozen bank.
pub fn generate_stake_meta_collection(
    bank: Arc<Bank>,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
) -> Result<StakeMetaCollection, StakeMetaError> {
    if !bank.is_frozen() {
        return Err(StakeMetaError::NotFrozen(bank.slot()));
    }

    let epoch = bank.epoch();
    let vote_accounts = bank
        .epoch_vote_accounts(epoch)
        .ok_or_else(|| StakeMetaError::NoVoteAccounts(bank.slot(), epoch))?;
    let stake_history = stake_history(&bank);
    let delegations = collect_delegations(&bank, &stake_history)?;
    let (tip_receiver, tip_receiver_fee) = tip_receiver_info(&bank, tip_payment_program_id)?;

    let mut stake_metas = vote_accounts
        .iter()
        .filter_map(|(vote_pubkey, (_, vote_account))| {
            let mut voter_delegations = delegations.get(vote_pubkey)?.clone();
            voter_delegations.sort_unstable();
            let vote_state = vote_account.vote_state_view();
            let total_delegated = voter_delegations
                .iter()
                .try_fold(0u64, |total, delegation| {
                    total.checked_add(delegation.lamports_delegated)
                })?;

            Some(StakeMeta {
                maybe_tip_distribution_meta: distribution_meta::<
                    TipDistributionAccount,
                    TipDistributionMeta,
                >(
                    &bank,
                    tip_distribution_program_id,
                    derive_tip_distribution_account_address(
                        tip_distribution_program_id,
                        vote_pubkey,
                        epoch,
                    )
                    .0,
                    Some((tip_receiver, tip_receiver_fee)),
                    |account, account_data, address, rent_exempt_amount| {
                        Some(TipDistributionMeta {
                            tip_distribution_pubkey: address,
                            total_tips: account_data.lamports().checked_sub(rent_exempt_amount)?,
                            validator_fee_bps: account.validator_commission_bps,
                            merkle_root_upload_authority: account.merkle_root_upload_authority,
                        })
                    },
                ),
                maybe_priority_fee_distribution_meta: distribution_meta::<
                    PriorityFeeDistributionAccount,
                    PriorityFeeDistributionMeta,
                >(
                    &bank,
                    priority_fee_distribution_program_id,
                    derive_priority_fee_distribution_account_address(
                        priority_fee_distribution_program_id,
                        vote_pubkey,
                        epoch,
                    )
                    .0,
                    None,
                    |account, account_data, address, rent_exempt_amount| {
                        Some(PriorityFeeDistributionMeta {
                            priority_fee_distribution_pubkey: address,
                            total_tips: account_data.lamports().checked_sub(rent_exempt_amount)?,
                            validator_fee_bps: account.validator_commission_bps,
                            merkle_root_upload_authority: account.merkle_root_upload_authority,
                        })
                    },
                ),
                validator_node_pubkey: *vote_state.node_pubkey(),
                validator_vote_account: *vote_pubkey,
                delegations: voter_delegations,
                total_delegated,
                commission: vote_state.commission(),
            })
        })
        .collect::<Vec<_>>();
    stake_metas.sort();

    Ok(StakeMetaCollection {
        stake_metas,
        tip_distribution_program_id: *tip_distribution_program_id,
        priority_fee_distribution_program_id: *priority_fee_distribution_program_id,
        bank_hash: bank.hash().to_string(),
        epoch,
        slot: bank.slot(),
    })
}

fn stake_history(bank: &Bank) -> StakeHistory {
    let account = bank
        .get_account(&stake_history::id())
        .expect("stake history sysvar account should be present in the loaded bank");
    bincode::deserialize(account.data()).expect("stake history sysvar account should deserialize")
}

fn collect_delegations(
    bank: &Bank,
    stake_history: &StakeHistory,
) -> Result<HashMap<Pubkey, Vec<Delegation>>, StakeMetaError> {
    // `unfiltered_stakes()` is unavailable in this workspace's v4.2 runtime.
    // Use the snapshot-service calculation's direct-account fallback instead.
    let stake_program_id = solana_stake_interface::program::id();
    let mut accounts = bank
        .get_filtered_indexed_accounts(&IndexKey::ProgramId(stake_program_id), |_| true, None)
        .map_err(|error| StakeMetaError::ScanError(format!("{error:?}")))?;
    if accounts.is_empty() {
        warn!("ProgramId index returned no stake accounts; falling back to full program scan");
        accounts = bank
            .get_program_accounts(&stake_program_id)
            .map_err(|error| StakeMetaError::ScanError(format!("{error:?}")))?;
    }
    let stake_accounts = accounts
        .into_iter()
        .filter_map(|(address, account)| {
            StakeAccount::try_from(account)
                .ok()
                .map(|stake_account| (address, stake_account))
        })
        .collect::<Vec<_>>();
    collect_delegations_from_accounts(
        bank,
        stake_history,
        stake_accounts
            .iter()
            .map(|(stake_pubkey, stake_account)| (stake_pubkey, stake_account)),
    )
}

fn collect_delegations_from_accounts<'a>(
    bank: &Bank,
    stake_history: &StakeHistory,
    stake_accounts: impl IntoIterator<Item = (&'a Pubkey, &'a StakeAccount)>,
) -> Result<HashMap<Pubkey, Vec<Delegation>>, StakeMetaError> {
    let mut delegations = HashMap::<Pubkey, Vec<Delegation>>::new();
    for (stake_pubkey, stake_account) in stake_accounts {
        let delegation = stake_account.delegation();
        if delegation.stake_v2(
            bank.epoch(),
            stake_history,
            bank.new_warmup_cooldown_rate_epoch(),
        ) == 0
        {
            continue;
        }

        let authorized = stake_account.stake_state().authorized().unwrap_or_default();
        delegations
            .entry(delegation.voter_pubkey)
            .or_default()
            .push(Delegation {
                stake_account_pubkey: *stake_pubkey,
                staker_pubkey: authorized.staker,
                withdrawer_pubkey: authorized.withdrawer,
                lamports_delegated: delegation.stake,
            });
    }
    Ok(delegations)
}

fn tip_receiver_info(
    bank: &Bank,
    tip_payment_program_id: &Pubkey,
) -> Result<(Pubkey, u64), StakeMetaError> {
    let config_address =
        Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], tip_payment_program_id).0;
    let config = bank
        .get_account(&config_address)
        .ok_or_else(|| StakeMetaError::AnchorError("Config account not found in bank".into()))
        .and_then(|account| {
            Config::deserialize(account.data())
                .map_err(|_| StakeMetaError::AnchorError("Failed to deserialize config".into()))
        })?;

    let excess_tip_balances = TIP_ACCOUNT_SEEDS.iter().try_fold(0u64, |total, seed| {
        let address = Pubkey::find_program_address(&[*seed], tip_payment_program_id).0;
        let account = bank.get_account(&address).expect("tip account exists");
        let balance = account
            .lamports()
            .checked_sub(bank.get_minimum_balance_for_rent_exemption(account.data().len()))
            .ok_or(StakeMetaError::CheckedMathError)?;
        total
            .checked_add(balance)
            .ok_or(StakeMetaError::CheckedMathError)
    })?;
    let block_builder_tips = excess_tip_balances
        .checked_mul(config.block_builder_commission_pct)
        .and_then(|tips| tips.checked_div(100))
        .ok_or(StakeMetaError::CheckedMathError)?;
    let tip_receiver_fee = excess_tip_balances
        .checked_sub(block_builder_tips)
        .ok_or(StakeMetaError::CheckedMathError)?;
    Ok((config.tip_receiver, tip_receiver_fee))
}

fn distribution_meta<DistributionAccount, Meta>(
    bank: &Bank,
    program_id: &Pubkey,
    address: Pubkey,
    tip_receiver_info: Option<(Pubkey, u64)>,
    build: impl FnOnce(DistributionAccount, AccountSharedData, Pubkey, u64) -> Option<Meta>,
) -> Option<Meta>
where
    DistributionAccount: BorshDeserialize,
{
    let mut account_data: AccountSharedData = bank.get_account(&address)?;
    if account_data.owner() != program_id {
        return None;
    }
    let account = DistributionAccount::deserialize(&mut &account_data.data().get(8..)?[..]).ok()?;
    if let Some((tip_receiver, fee)) = tip_receiver_info {
        if address == tip_receiver {
            account_data.set_lamports(account_data.lamports().checked_add(fee)?);
        }
    }
    let expected_len = 8usize.checked_add(size_of::<DistributionAccount>())?;
    if account_data.data().len() != expected_len {
        warn!(
            "distribution account length mismatch: actual={}, expected={expected_len}",
            account_data.data().len()
        );
    }
    let rent_exempt_amount = bank.get_minimum_balance_for_rent_exemption(account_data.data().len());
    build(account, account_data, address, rent_exempt_amount)
}

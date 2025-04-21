use std::sync::Arc;

use anchor_lang::AccountDeserialize;
use jito_priority_fee_distribution_sdk::{
    derive_priority_fee_distribution_account_address, PriorityFeeDistributionAccount,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, TipDistributionAccount};
use meta_merkle_tree::generated_merkle_tree::{PriorityFeeDistributionMeta, TipDistributionMeta};
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    clock::Epoch,
    pubkey::Pubkey,
};

use crate::stake_meta_generator::StakeMetaGeneratorError;

pub trait DistributionWrapper {
    type DistributionAccountType;

    fn new_from_account(
        distribution_account: Self::DistributionAccountType,
        acount_data: AccountSharedData,
        pubkey: Pubkey,
    ) -> Self;

    fn derive_distribution_account_address(
        program_id: &Pubkey,
        vote_pubkey: &Pubkey,
        epoch: Epoch,
    ) -> Pubkey;
}

/// Convenience wrapper around [TipDistributionAccount]
pub struct TipDistributionAccountWrapper {
    pub tip_distribution_account: TipDistributionAccount,
    pub account_data: AccountSharedData,
    pub tip_distribution_pubkey: Pubkey,
}
impl DistributionWrapper for TipDistributionAccountWrapper {
    type DistributionAccountType = TipDistributionAccount;

    fn new_from_account(
        distribution_account: TipDistributionAccount,
        acount_data: AccountSharedData,
        pubkey: Pubkey,
    ) -> Self {
        Self {
            tip_distribution_account: distribution_account,
            account_data: acount_data,
            tip_distribution_pubkey: pubkey,
        }
    }

    fn derive_distribution_account_address(
        program_id: &Pubkey,
        vote_pubkey: &Pubkey,
        epoch: Epoch,
    ) -> Pubkey {
        derive_tip_distribution_account_address(program_id, vote_pubkey, epoch).0
    }
}

/// Convenience wrapper around [PriorityFeeDistributionAccount]
pub struct PriorityFeeDistributionAccountWrapper {
    pub priority_fee_distribution_account: PriorityFeeDistributionAccount,
    pub account_data: AccountSharedData,
    pub priority_fee_distribution_pubkey: Pubkey,
}
impl DistributionWrapper for PriorityFeeDistributionAccountWrapper {
    type DistributionAccountType = PriorityFeeDistributionAccount;

    fn new_from_account(
        distribution_account: PriorityFeeDistributionAccount,
        acount_data: AccountSharedData,
        pubkey: Pubkey,
    ) -> Self {
        Self {
            priority_fee_distribution_account: distribution_account,
            account_data: acount_data,
            priority_fee_distribution_pubkey: pubkey,
        }
    }

    fn derive_distribution_account_address(
        program_id: &Pubkey,
        vote_pubkey: &Pubkey,
        epoch: Epoch,
    ) -> Pubkey {
        derive_priority_fee_distribution_account_address(program_id, vote_pubkey, epoch).0
    }
}

pub struct TipReceiverInfo {
    pub tip_receiver: Pubkey,
    pub tip_receiver_fee: u64,
}

pub fn get_distribution_account<T, R>(
    bank: &Arc<Bank>,
    program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    tip_receiver_info: Option<TipReceiverInfo>,
) -> Option<R>
where
    T: AccountDeserialize,
    R: DistributionWrapper<DistributionAccountType = T>,
{
    let distribution_account_pubkey =
        R::derive_distribution_account_address(program_id, vote_pubkey, bank.epoch());
    bank.get_account(&distribution_account_pubkey).map_or_else(
        || None,
        |mut account_data| {
            // DAs may be funded with lamports and therefore exist in the bank, but would fail the
            // deserialization step if the buffer is yet to be allocated thru the init call to the
            // program.
            T::try_deserialize(&mut account_data.data()).map_or_else(
                |_| None,
                |distribution_account| {
                    // [TIp Distribution ONLY] this snapshot might have tips that weren't claimed
                    // by the time the epoch is over assume that it will eventually be cranked and
                    // credit the excess to this account
                    if let Some(tip_receiver_info) = tip_receiver_info {
                        if distribution_account_pubkey == tip_receiver_info.tip_receiver {
                            account_data.set_lamports(
                                account_data
                                    .lamports()
                                    .checked_add(tip_receiver_info.tip_receiver_fee)
                                    .expect("tip overflow"),
                            );
                        }
                    }

                    Some(R::new_from_account(
                        distribution_account,
                        account_data,
                        distribution_account_pubkey,
                    ))
                },
            )
        },
    )
}

pub fn tip_distribution_account_from_tda_wrapper(
    tda_wrapper: TipDistributionAccountWrapper,
    // The amount that will be left remaining in the tda to maintain rent exemption status.
    rent_exempt_amount: u64,
) -> Result<TipDistributionMeta, StakeMetaGeneratorError> {
    Ok(TipDistributionMeta {
        tip_distribution_pubkey: tda_wrapper.tip_distribution_pubkey,
        total_tips: tda_wrapper
            .account_data
            .lamports()
            .checked_sub(rent_exempt_amount)
            .ok_or(StakeMetaGeneratorError::CheckedMathError)?,
        validator_fee_bps: tda_wrapper
            .tip_distribution_account
            .validator_commission_bps,
        merkle_root_upload_authority: tda_wrapper
            .tip_distribution_account
            .merkle_root_upload_authority,
    })
}

/// Converts the `PriorityFeeDistributionAccountWrapper` to StakeMeta's exptected `PriorityFeeDistributionMeta`
pub fn pf_tip_distribution_account_from_tda_wrapper(
    pf_distribution_account_wrapper: PriorityFeeDistributionAccountWrapper,
    // The amount that will be left remaining in the tda to maintain rent exemption status.
    rent_exempt_amount: u64,
) -> Result<PriorityFeeDistributionMeta, StakeMetaGeneratorError> {
    Ok(PriorityFeeDistributionMeta {
        priority_fee_distribution_pubkey: pf_distribution_account_wrapper
            .priority_fee_distribution_pubkey,
        total_tips: pf_distribution_account_wrapper
            .account_data
            .lamports()
            .checked_sub(rent_exempt_amount)
            .ok_or(StakeMetaGeneratorError::CheckedMathError)?,
        validator_fee_bps: pf_distribution_account_wrapper
            .priority_fee_distribution_account
            .validator_commission_bps,
        merkle_root_upload_authority: pf_distribution_account_wrapper
            .priority_fee_distribution_account
            .merkle_root_upload_authority,
    })
}

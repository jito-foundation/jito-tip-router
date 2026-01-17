use std::sync::Arc;

use borsh::de::BorshDeserialize;
use jito_priority_fee_distribution_sdk::{
    derive_priority_fee_distribution_account_address, PriorityFeeDistributionAccount,
};
use jito_tip_distribution_sdk::{derive_tip_distribution_account_address, TipDistributionAccount};
use log::warn;
use meta_merkle_tree::generated_merkle_tree::{PriorityFeeDistributionMeta, TipDistributionMeta};
use solana_runtime::bank::Bank;
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount, WritableAccount},
    clock::Epoch,
    pubkey::Pubkey,
};

use crate::stake_meta_generator::StakeMetaGeneratorError;

pub trait DistributionMeta {
    type DistributionAccountType;

    fn new_from_account(
        distribution_account: Self::DistributionAccountType,
        account_data: AccountSharedData,
        pubkey: Pubkey,
        rent_exempt_amount: u64,
    ) -> Result<Self, StakeMetaGeneratorError>
    where
        Self: Sized;

    fn derive_distribution_account_address(
        program_id: &Pubkey,
        vote_pubkey: &Pubkey,
        epoch: Epoch,
    ) -> Pubkey;
}

pub struct WrappedTipDistributionMeta(pub TipDistributionMeta);
impl DistributionMeta for WrappedTipDistributionMeta {
    type DistributionAccountType = TipDistributionAccount;

    fn new_from_account(
        distribution_account: Self::DistributionAccountType,
        account_data: AccountSharedData,
        pubkey: Pubkey,
        rent_exempt_amount: u64,
    ) -> Result<Self, StakeMetaGeneratorError> {
        Ok(Self(TipDistributionMeta {
            tip_distribution_pubkey: pubkey,
            total_tips: account_data
                .lamports()
                .checked_sub(rent_exempt_amount)
                .ok_or(StakeMetaGeneratorError::CheckedMathError)?,
            validator_fee_bps: distribution_account.validator_commission_bps,
            merkle_root_upload_authority: distribution_account.merkle_root_upload_authority,
        }))
    }

    fn derive_distribution_account_address(
        program_id: &Pubkey,
        vote_pubkey: &Pubkey,
        epoch: Epoch,
    ) -> Pubkey {
        derive_tip_distribution_account_address(program_id, vote_pubkey, epoch).0
    }
}

pub struct WrappedPriorityFeeDistributionMeta(pub PriorityFeeDistributionMeta);
impl DistributionMeta for WrappedPriorityFeeDistributionMeta {
    type DistributionAccountType = PriorityFeeDistributionAccount;

    fn new_from_account(
        distribution_account: Self::DistributionAccountType,
        account_data: AccountSharedData,
        pubkey: Pubkey,
        rent_exempt_amount: u64,
    ) -> Result<Self, StakeMetaGeneratorError> {
        Ok(Self(PriorityFeeDistributionMeta {
            priority_fee_distribution_pubkey: pubkey,
            total_tips: account_data
                .lamports()
                .checked_sub(rent_exempt_amount)
                .ok_or(StakeMetaGeneratorError::CheckedMathError)?,
            validator_fee_bps: distribution_account.validator_commission_bps,
            merkle_root_upload_authority: distribution_account.merkle_root_upload_authority,
        }))
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

pub fn get_distribution_meta<DistributionAccount, DistMeta>(
    bank: &Arc<Bank>,
    program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    tip_receiver_info: Option<TipReceiverInfo>,
) -> Option<DistMeta>
where
    DistributionAccount: BorshDeserialize,
    DistMeta: DistributionMeta<DistributionAccountType = DistributionAccount>,
{
    let distribution_account_pubkey =
        DistMeta::derive_distribution_account_address(program_id, vote_pubkey, bank.epoch());
    bank.get_account(&distribution_account_pubkey).map_or_else(
        || None,
        |mut account_data| {
            if account_data.owner() != program_id {
                return None;
            }
            // DAs may be funded with lamports and therefore exist in the bank, but would fail the
            // deserialization step if the buffer is yet to be allocated thru the init call to the
            // program.
            let Some(distribution_account_data) = account_data.data().get(8..) else {
                return None;
            };
            DistributionAccount::deserialize(&mut distribution_account_data.as_ref()).map_or_else(
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

                    let actual_len = account_data.data().len();
                    let expected_len = 8_usize.saturating_add(size_of::<DistributionAccount>());
                    if actual_len != expected_len {
                        warn!("len mismatch actual={actual_len}, expected={expected_len}");
                    }
                    let rent_exempt_amount =
                        bank.get_minimum_balance_for_rent_exemption(account_data.data().len());

                    DistMeta::new_from_account(
                        distribution_account,
                        account_data,
                        distribution_account_pubkey,
                        rent_exempt_amount,
                    )
                    .ok()
                },
            )
        },
    )
}

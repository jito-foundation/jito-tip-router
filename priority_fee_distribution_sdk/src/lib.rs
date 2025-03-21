#![allow(clippy::redundant_pub_crate)]
use anchor_lang::{declare_program, prelude::Pubkey, solana_program::clock::Epoch};

declare_program!(jito_priority_fee_distribution);
pub use jito_priority_fee_distribution::accounts::PriorityFeeDistributionAccount;

pub mod instruction;

pub const CONFIG_SEED: &[u8] = b"CONFIG_ACCOUNT";
pub const CLAIM_STATUS_SEED: &[u8] = b"CLAIM_STATUS";
pub const PF_DISTRIBUTION_SEED: &[u8] = b"PF_DISTRIBUTION_ACCOUNT";
pub const MERKLE_ROOT_UPLOAD_CONFIG_SEED: &[u8] = b"ROOT_UPLOAD_CONFIG";

pub const HEADER_SIZE: usize = 8;
pub const TIP_DISTRIBUTION_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<PriorityFeeDistributionAccount>();
pub const CLAIM_STATUS_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<jito_priority_fee_distribution::accounts::ClaimStatus>();
pub const CONFIG_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<jito_priority_fee_distribution::accounts::Config>();

pub fn derive_priority_fee_distribution_account_address(
    priority_fee_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            PF_DISTRIBUTION_SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        priority_fee_distribution_program_id,
    )
}

pub fn derive_config_account_address(
    priority_fee_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED], priority_fee_distribution_program_id)
}

pub fn derive_claim_status_account_address(
    priority_fee_distribution_program_id: &Pubkey,
    claimant: &Pubkey,
    priority_fee_distribution_account: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            CLAIM_STATUS_SEED,
            claimant.to_bytes().as_ref(),
            priority_fee_distribution_account.to_bytes().as_ref(),
        ],
        priority_fee_distribution_program_id,
    )
}

pub fn derive_merkle_root_upload_authority_address(
    priority_fee_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MERKLE_ROOT_UPLOAD_CONFIG_SEED],
        priority_fee_distribution_program_id,
    )
}

pub fn id() -> Pubkey {
    jito_priority_fee_distribution::ID
}

pub mod instruction;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{epoch_schedule::Epoch, pubkey::Pubkey};
use std::str::FromStr;

pub const CONFIG_SEED: &[u8] = b"CONFIG_ACCOUNT";
pub const CLAIM_STATUS_SEED: &[u8] = b"CLAIM_STATUS";
pub const PF_DISTRIBUTION_SEED: &[u8] = b"PF_DISTRIBUTION_ACCOUNT";
pub const MERKLE_ROOT_UPLOAD_CONFIG_SEED: &[u8] = b"ROOT_UPLOAD_CONFIG";

pub const HEADER_SIZE: usize = 8;
pub const PRIORITY_FEE_DISTRIBUTION_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<PriorityFeeDistributionAccount>();
pub const CLAIM_STATUS_SIZE: usize = HEADER_SIZE + std::mem::size_of::<ClaimStatus>();
pub const CONFIG_SIZE: usize = HEADER_SIZE + std::mem::size_of::<Config>();

#[derive(BorshDeserialize, BorshSerialize)]
pub struct Config {
    /// Account with authority over this PDA.
    pub authority: Pubkey,

    /// We want to expire funds after some time so that validators can be refunded the rent.
    /// Expired funds will get transferred to this account.
    pub expired_funds_account: Pubkey,

    /// Specifies the number of epochs a merkle root is valid for before expiring.
    pub num_epochs_valid: u64,

    /// The maximum commission a validator can set on their distribution account.
    pub max_validator_commission_bps: u16,

    /// The epoch where lamports are transferred to the priority fee distribution account.
    pub go_live_epoch: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

impl Config {
    pub const DISCRIMINATOR: [u8; 8] = [0; 8];
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ClaimStatus {
    /// The account that pays the rent for this account
    pub claim_status_payer: Pubkey,

    /// The epoch (upto and including) that tip funds can be claimed.
    /// Copied since TDA can be closed, need to track to avoid making multiple claims
    pub expires_at: u64,
}

impl ClaimStatus {
    pub const DISCRIMINATOR: [u8; 8] = [0; 8];
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct PriorityFeeDistributionAccount {
    /// The validator's vote account, also the recipient of remaining lamports after
    /// upon closing this account.
    pub validator_vote_account: Pubkey,

    /// The only account authorized to upload a merkle-root for this account.
    pub merkle_root_upload_authority: Pubkey,

    /// The merkle root used to verify user claims from this account.
    pub merkle_root: Option<MerkleRoot>,

    /// Epoch for which this account was created.
    pub epoch_created_at: u64,

    /// The commission basis points this validator charges.
    pub validator_commission_bps: u16,

    /// The epoch (upto and including) that tip funds can be claimed.
    pub expires_at: u64,

    /// The total lamports transferred to this account.
    pub total_lamports_transferred: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

impl PriorityFeeDistributionAccount {
    pub const DISCRIMINATOR: [u8; 8] = [0; 8];
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct MerkleRoot {
    /// The 256-bit merkle root.
    pub root: [u8; 32],

    /// Maximum number of funds that can ever be claimed from this [MerkleRoot].
    pub max_total_claim: u64,

    /// Maximum number of nodes that can ever be claimed from this [MerkleRoot].
    pub max_num_nodes: u64,

    /// Total funds that have been claimed.
    pub total_funds_claimed: u64,

    /// Number of nodes that have been claimed.
    pub num_nodes_claimed: u64,
}

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
    Pubkey::from_str("Priority6weCZ5HwDn29NxLFpb7TDp2iLZ6XKc5e8d3").unwrap()
}

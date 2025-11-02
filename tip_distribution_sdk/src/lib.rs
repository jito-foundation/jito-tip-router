use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
pub use solana_program::epoch_schedule::Epoch;
pub use solana_pubkey::Pubkey;
pub mod instruction;

use std::str::FromStr;

pub const CONFIG_SEED: &[u8] = b"CONFIG_ACCOUNT";
pub const CLAIM_STATUS_SEED: &[u8] = b"CLAIM_STATUS";
pub const TIP_DISTRIBUTION_SEED: &[u8] = b"TIP_DISTRIBUTION_ACCOUNT";
pub const MERKLE_ROOT_UPLOAD_CONFIG_SEED: &[u8] = b"ROOT_UPLOAD_CONFIG";

pub const HEADER_SIZE: usize = 8;
// Expected size: 168
pub const TIP_DISTRIBUTION_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<TipDistributionAccount>();
// Expected size: 104
pub const CLAIM_STATUS_SIZE: usize = HEADER_SIZE + std::mem::size_of::<ClaimStatus>();
// Expected size: 88
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

    /// The bump used to generate this account
    pub bump: u8,
}

impl Config {
    pub const DISCRIMINATOR: [u8; 8] = [0x9b, 0x0c, 0xaa, 0xe0, 0x1e, 0xfa, 0xcc, 82];

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        anyhow::ensure!(data.len() >= 8, "Account data too short");
        anyhow::ensure!(data.len() >= CONFIG_SIZE, "Invalid account size");
        let (discriminator, mut remainder) = data.split_at(8);
        anyhow::ensure!(
            discriminator == Self::DISCRIMINATOR,
            "Invalid discriminator"
        );
        Ok(<Self as BorshDeserialize>::deserialize(&mut remainder)?)
    }
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

#[derive(BorshDeserialize, BorshSerialize)]
pub struct TipDistributionAccount {
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

    /// The bump used to generate this account
    pub bump: u8,
}

impl TipDistributionAccount {
    pub const DISCRIMINATOR: [u8; 8] = [0x55, 0x40, 0x71, 0xc6, 0xea, 0x5e, 0x78, 0x7b];

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        anyhow::ensure!(data.len() >= 8, "Account data too short");
        anyhow::ensure!(data.len() >= TIP_DISTRIBUTION_SIZE, "Invalid account size");
        let (discriminator, mut remainder) = data.split_at(8);
        anyhow::ensure!(
            discriminator == Self::DISCRIMINATOR,
            "Invalid discriminator"
        );
        Ok(<Self as BorshDeserialize>::deserialize(&mut remainder)?)
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct ClaimStatus {
    /// If true, the tokens have been claimed.
    pub is_claimed: bool,

    /// Authority that claimed the tokens. Allows for delegated rewards claiming.
    pub claimant: Pubkey,

    /// The payer who created the claim.
    pub claim_status_payer: Pubkey,

    /// When the funds were claimed.
    pub slot_claimed_at: u64,

    /// Amount of funds claimed.
    pub amount: u64,

    /// The epoch (upto and including) that tip funds can be claimed.
    /// Copied since TDA can be closed, need to track to avoid making multiple claims
    pub expires_at: u64,

    /// The bump used to generate this account
    pub bump: u8,
}

impl ClaimStatus {
    pub const DISCRIMINATOR: [u8; 8] = [22, 183, 249, 157, 247, 95, 150, 96];

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        anyhow::ensure!(data.len() >= 8, "Account data too short");
        anyhow::ensure!(data.len() >= CLAIM_STATUS_SIZE, "Invalid account size");
        let (discriminator, mut remainder) = data.split_at(8);
        anyhow::ensure!(
            discriminator == Self::DISCRIMINATOR,
            "Invalid discriminator"
        );
        Ok(<Self as BorshDeserialize>::deserialize(&mut remainder)?)
    }
}

pub fn derive_tip_distribution_account_address(
    tip_distribution_program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    epoch: Epoch,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            TIP_DISTRIBUTION_SEED,
            vote_pubkey.to_bytes().as_ref(),
            epoch.to_le_bytes().as_ref(),
        ],
        tip_distribution_program_id,
    )
}

pub fn derive_config_account_address(tip_distribution_program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG_SEED], tip_distribution_program_id)
}

pub fn derive_claim_status_account_address(
    tip_distribution_program_id: &Pubkey,
    claimant: &Pubkey,
    tip_distribution_account: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            CLAIM_STATUS_SEED,
            claimant.to_bytes().as_ref(),
            tip_distribution_account.to_bytes().as_ref(),
        ],
        tip_distribution_program_id,
    )
}

pub fn derive_merkle_root_upload_authority_address(
    tip_distribution_program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[MERKLE_ROOT_UPLOAD_CONFIG_SEED],
        tip_distribution_program_id,
    )
}

pub fn id() -> Pubkey {
    Pubkey::from_str("4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7")
        .expect("Failed to parse program id")
}

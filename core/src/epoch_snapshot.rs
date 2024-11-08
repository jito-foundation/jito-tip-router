use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{
    types::{PodU128, PodU16, PodU64},
    AccountDeserialize, Discriminator,
};
use jito_vault_core::delegation_state::DelegationState;
use shank::{ShankAccount, ShankType};
use solana_program::pubkey::Pubkey;

use crate::{discriminators::Discriminators, fees::Fees};

// PDA'd ["EPOCH_SNAPSHOT", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochSnapshot {
    /// The NCN on-chain program is the signer to create and update this account,
    /// this pushes the responsibility of managing the account to the NCN program.
    ncn: Pubkey,

    /// The NCN epoch for which the Epoch snapshot is valid
    ncn_epoch: PodU64,

    /// Slot Epoch snapshot was created
    slot_created: PodU64,

    /// Bump seed for the PDA
    bump: u8,

    /// Reserved space
    reserved: [u8; 128],

    ncn_fees: Fees,

    num_operators: PodU16,
    operators_registered: PodU16,

    /// Counted as each delegate gets added
    total_votes: PodU128,
}

impl Discriminator for EpochSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

// PDA'd ["OPERATOR_SNAPSHOT", OPERATOR, NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorSnapshot {
    operator: Pubkey,
    ncn: Pubkey,
    ncn_epoch: PodU64,
    slot_created: PodU64,

    bump: u8,

    operator_fee_bps: PodU16,

    total_votes: PodU128,

    num_vault_operator_delegations: PodU16,
    vault_operator_delegations_registered: PodU16,

    slot_set: PodU64,
    vault_operator_delegations: [VaultOperatorDelegationSnapshot; 256],
}

impl Discriminator for OperatorSnapshot {
    const DISCRIMINATOR: u8 = Discriminators::OperatorSnapshot as u8;
}

// Operators effectively cast N types of votes,
// where N is the number of supported mints
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct VaultOperatorDelegationSnapshot {
    vault: Pubkey,
    st_mint: Pubkey,
    delegation_state: DelegationState,
    total_votes: PodU128,
    slot_set: PodU64,
    reserved: [u8; 128],
}

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::pubkey::Pubkey;

use crate::{
    ballot_box::{self, BallotBox},
    discriminators::Discriminators,
    error::TipRouterError,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct OperatorReward {
    operator: Pubkey,
    reward: PodU64,
    reserved: [u8; 128],
}

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochRewardRouter {
    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reserved: [u8; 128],

    reward_pool: PodU64,

    operator_rewards: [OperatorReward; 32],
}

impl Discriminator for EpochRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochRewardRouter {
    pub fn process_new_rewards(
        &mut self,
        ballot_box: &BallotBox,
        new_rewards: u64,
    ) -> Result<(), TipRouterError> {
        let winning_ballot = ballot_box.get_winning_ballot()?;
        for tally in ballot_box.ballot_tallies() {}

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct VaultReward {
    vault: Pubkey,
    reward: PodU64,
    reserved: [u8; 128],
}

// PDA'd ["operator_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct OperatorRewardRouter {
    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reserved: [u8; 128],

    reward_pool: PodU64,

    vault_rewards: [VaultReward; 32],
}

impl Discriminator for OperatorRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochSnapshot as u8;
}

impl EpochRewardRouter {}

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use jito_vault_core::MAX_BPS;
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};
use spl_math::precise_number::PreciseNumber;

use crate::{
    ballot_box::{self, BallotBox, OperatorVote},
    discriminators::Discriminators,
    epoch_snapshot::{OperatorSnapshot, VaultOperatorStakeWeight},
    error::TipRouterError,
    fees::FeeConfig,
    ncn_fee_group::NcnFeeGroup,
};

#[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
#[repr(C)]
pub struct RewardRoutes {
    destination: Pubkey,
    rewards: PodU64,
    reserved: [u8; 128],
}

impl Default for RewardRoutes {
    fn default() -> Self {
        Self {
            destination: Pubkey::default(),
            rewards: PodU64::from(0),
            reserved: [0; 128],
        }
    }
}

impl RewardRoutes {
    pub const fn destination(&self) -> Pubkey {
        self.destination
    }

    pub fn rewards(&self) -> u64 {
        self.rewards.into()
    }

    pub fn increment_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.rewards = PodU64::from(
            self.rewards()
                .checked_add(rewards)
                .ok_or(TipRouterError::ArithmeticOverflow)?,
        );
        Ok(())
    }

    pub fn decrement_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
        self.rewards = PodU64::from(
            self.rewards()
                .checked_sub(rewards)
                .ok_or(TipRouterError::ArithmeticUnderflowError)?,
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct RewardBucket {
    rewards: PodU64,
    reserved: [u8; 64],
}

impl Default for RewardBucket {
    fn default() -> Self {
        Self {
            rewards: PodU64::from(0),
            reserved: [0; 64],
        }
    }
}

// PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochRewardRouter {
    ncn: Pubkey,

    ncn_epoch: PodU64,

    bump: u8,

    slot_created: PodU64,

    reward_pool: PodU64,

    doa_rewards: PodU64,

    reserved: [u8; 128],

    ncn_reward_buckets: [RewardBucket; 16],

    //TODO change to 256
    reward_routes: [RewardRoutes; 32],
}

impl Discriminator for EpochRewardRouter {
    const DISCRIMINATOR: u8 = Discriminators::EpochRewardRouter as u8;
}

impl EpochRewardRouter {
    pub fn new(ncn: Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
        Self {
            ncn,
            ncn_epoch: PodU64::from(ncn_epoch),
            bump,
            slot_created: PodU64::from(slot_created),
            ncn_reward_buckets: [RewardBucket::default(); NcnFeeGroup::FEE_GROUP_COUNT],
            reward_routes: [RewardRoutes::default(); 32],
            reward_pool: PodU64::from(0),
            doa_rewards: PodU64::from(0),
            reserved: [0; 128],
        }
    }

    pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
        Vec::from_iter(
            [
                b"epoch_reward_router".to_vec(),
                ncn.to_bytes().to_vec(),
                ncn_epoch.to_le_bytes().to_vec(),
            ]
            .iter()
            .cloned(),
        )
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn, ncn_epoch);
        let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
        let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
        (pda, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_epoch: u64,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(program_id) {
            msg!("Epoch Reward Router account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_is_empty() {
            msg!("Epoch Reward Router account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !account.is_writable {
            msg!("Epoch Reward Router account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("Epoch Reward Router account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if account
            .key
            .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
        {
            msg!("Epoch Reward Router account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn process_reward_pool(
        &mut self,
        fee_config: &FeeConfig,
        ballot_box: &BallotBox,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        let base_fee = fee_config.adjusted_dao_fee_bps(current_epoch)?;

        let winning_ballot = ballot_box.get_winning_ballot()?;

        let winning_stake_weight = winning_ballot.stake_weight();

        // DAO Fee
        {
            let rewards_to_process: u64 = self.reward_pool();

            let dao_rewards = Self::calculate_dao_rewards(base_fee, rewards_to_process)?;

            self.increment_dao_rewards(dao_rewards)?;
            self.decrement_reward_pool(dao_rewards)?;
        }
    }
}

// // PDA'd ["epoch_reward_router", NCN, NCN_EPOCH_SLOT]
// #[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
// #[repr(C)]
// pub struct EpochRewardRouter {
//     ncn: Pubkey,

//     ncn_epoch: PodU64,

//     bump: u8,

//     slot_created: PodU64,
//     /// base_rewards == dao_rewards
//     router: RewardRouter,
// }

// impl Discriminator for EpochRewardRouter {
//     const DISCRIMINATOR: u8 = Discriminators::EpochRewardRouter as u8;
// }

// impl EpochRewardRouter {
//     pub fn new(ncn: Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
//         Self {
//             ncn,
//             ncn_epoch: PodU64::from(ncn_epoch),
//             bump,
//             slot_created: PodU64::from(slot_created),
//             router: RewardRouter::new(),
//         }
//     }

//     pub fn seeds(ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
//         Vec::from_iter(
//             [
//                 b"epoch_reward_router".to_vec(),
//                 ncn.to_bytes().to_vec(),
//                 ncn_epoch.to_le_bytes().to_vec(),
//             ]
//             .iter()
//             .cloned(),
//         )
//     }

//     pub fn find_program_address(
//         program_id: &Pubkey,
//         ncn: &Pubkey,
//         ncn_epoch: u64,
//     ) -> (Pubkey, u8, Vec<Vec<u8>>) {
//         let seeds = Self::seeds(ncn, ncn_epoch);
//         let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
//         let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
//         (pda, bump, seeds)
//     }

//     pub fn load(
//         program_id: &Pubkey,
//         ncn: &Pubkey,
//         ncn_epoch: u64,
//         account: &AccountInfo,
//         expect_writable: bool,
//     ) -> Result<(), ProgramError> {
//         if account.owner.ne(program_id) {
//             msg!("Epoch Reward Router account has an invalid owner");
//             return Err(ProgramError::InvalidAccountOwner);
//         }
//         if account.data_is_empty() {
//             msg!("Epoch Reward Router account data is empty");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if expect_writable && !account.is_writable {
//             msg!("Epoch Reward Router account is not writable");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
//             msg!("Epoch Reward Router account discriminator is invalid");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if account
//             .key
//             .ne(&Self::find_program_address(program_id, ncn, ncn_epoch).0)
//         {
//             msg!("Epoch Reward Router account is not at the correct PDA");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         Ok(())
//     }

//     pub fn process_reward_pool(
//         &mut self,
//         fee_config: &FeeConfig,
//         ballot_box: &BallotBox,
//         current_epoch: u64,
//     ) -> Result<(), TipRouterError> {
//         let base_fee = fee_config.adjusted_dao_fee_bps(current_epoch)?;

//         let winning_ballot = ballot_box.get_winning_ballot_tally()?;

//         let total_reward_stake_weight = winning_ballot.reward_stake_weight();

//         let reward_stake_weights: Vec<RewardStakeWeight> = ballot_box
//             .operator_votes()
//             .iter()
//             .filter_map(|vote| {
//                 if vote.ballot_index() == winning_ballot.index() {
//                     Some(RewardStakeWeight::from(vote))
//                 } else {
//                     None
//                 }
//             })
//             .collect();

//         self.router
//             .process_reward_pool(base_fee, total_reward_stake_weight, &reward_stake_weights)
//     }
// }

// // PDA'd ["operator_reward_router", OPERATOR, NCN, NCN_EPOCH_SLOT]
// #[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
// #[repr(C)]
// pub struct OperatorRewardRouter {
//     operator: Pubkey,

//     ncn: Pubkey,

//     ncn_epoch: PodU64,

//     bump: u8,

//     slot_created: PodU64,
//     /// base_rewards == operator_rewards
//     router: RewardRouter,
// }

// impl Discriminator for OperatorRewardRouter {
//     const DISCRIMINATOR: u8 = Discriminators::OperatorRewardRouter as u8;
// }

// impl OperatorRewardRouter {
//     pub fn new(operator: Pubkey, ncn: Pubkey, ncn_epoch: u64, bump: u8, slot_created: u64) -> Self {
//         Self {
//             operator,
//             ncn,
//             ncn_epoch: PodU64::from(ncn_epoch),
//             bump,
//             slot_created: PodU64::from(slot_created),
//             router: RewardRouter::new(),
//         }
//     }

//     pub fn seeds(operator: &Pubkey, ncn: &Pubkey, ncn_epoch: u64) -> Vec<Vec<u8>> {
//         Vec::from_iter(
//             [
//                 b"operator_reward_router".to_vec(),
//                 operator.to_bytes().to_vec(),
//                 ncn.to_bytes().to_vec(),
//                 ncn_epoch.to_le_bytes().to_vec(),
//             ]
//             .iter()
//             .cloned(),
//         )
//     }

//     pub fn find_program_address(
//         program_id: &Pubkey,
//         operator: &Pubkey,
//         ncn: &Pubkey,
//         ncn_epoch: u64,
//     ) -> (Pubkey, u8, Vec<Vec<u8>>) {
//         let seeds = Self::seeds(operator, ncn, ncn_epoch);
//         let seeds_iter: Vec<_> = seeds.iter().map(|s| s.as_slice()).collect();
//         let (pda, bump) = Pubkey::find_program_address(&seeds_iter, program_id);
//         (pda, bump, seeds)
//     }

//     pub fn load(
//         program_id: &Pubkey,
//         operator: &Pubkey,
//         ncn: &Pubkey,
//         ncn_epoch: u64,
//         account: &AccountInfo,
//         expect_writable: bool,
//     ) -> Result<(), ProgramError> {
//         if account.owner.ne(program_id) {
//             msg!("Operator Reward Router account has an invalid owner");
//             return Err(ProgramError::InvalidAccountOwner);
//         }
//         if account.data_is_empty() {
//             msg!("Operator Reward Router account data is empty");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if expect_writable && !account.is_writable {
//             msg!("Operator Reward Router account is not writable");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
//             msg!("Operator Reward Router account discriminator is invalid");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         if account
//             .key
//             .ne(&Self::find_program_address(program_id, operator, ncn, ncn_epoch).0)
//         {
//             msg!("Operator Reward Router account is not at the correct PDA");
//             return Err(ProgramError::InvalidAccountData);
//         }
//         Ok(())
//     }

//     pub fn process_reward_pool(
//         &mut self,
//         operator_snapshot: &OperatorSnapshot,
//     ) -> Result<(), TipRouterError> {
//         let base_fee = operator_snapshot.operator_fee_bps();

//         let total_reward_stake_weight = operator_snapshot.reward_stake_weight();

//         let reward_stake_weights: Vec<RewardStakeWeight> = operator_snapshot
//             .vault_operator_stake_weight()
//             .iter()
//             .map(|stake_weight| RewardStakeWeight::from(stake_weight))
//             .collect();

//         self.router.process_reward_pool(
//             base_fee as u64,
//             total_reward_stake_weight,
//             &reward_stake_weights,
//         )
//     }
// }

// #[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
// #[repr(C)]
// pub struct RewardRouter {
//     reward_pool: PodU64,

//     base_rewards: PodU64,

//     reward_routes: [RewardRoutes; 32],

//     reserved: [u8; 128],
// }

// impl RewardRouter {
//     pub fn new() -> Self {
//         Self {
//             reserved: [0; 128],
//             reward_pool: PodU64::from(0),
//             base_rewards: PodU64::from(0),
//             reward_routes: [RewardRoutes::default(); 32],
//         }
//     }

//     fn calculate_base_rewards(
//         base_fee: u64,
//         rewards_to_process: u64,
//     ) -> Result<u64, TipRouterError> {
//         let precise_base_fee =
//             PreciseNumber::new(base_fee as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
//             .ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_max_bps =
//             PreciseNumber::new(MAX_BPS as u128).ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_base_rewards = precise_rewards_to_process
//             .checked_mul(&precise_base_fee)
//             .and_then(|x| x.checked_div(&precise_max_bps))
//             .ok_or(TipRouterError::ArithmeticOverflow)?;

//         let floored_precise_base_rewards = precise_base_rewards
//             .floor()
//             .ok_or(TipRouterError::ArithmeticFloorError)?;

//         let base_rewards_u128: u128 = floored_precise_base_rewards
//             .to_imprecise()
//             .ok_or(TipRouterError::CastToImpreciseNumberError)?;

//         base_rewards_u128
//             .try_into()
//             .map_err(|_| TipRouterError::CastToU64Error)
//     }

//     fn calculate_route_rewards(
//         reward_stake_weight: u128,
//         total_reward_stake_weight: u128,
//         rewards_to_process: u64,
//     ) -> Result<u64, TipRouterError> {
//         let precise_reward_stake_weight =
//             PreciseNumber::new(reward_stake_weight).ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_total_reward_stake_weight = PreciseNumber::new(total_reward_stake_weight)
//             .ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_rewards_to_process = PreciseNumber::new(rewards_to_process as u128)
//             .ok_or(TipRouterError::NewPreciseNumberError)?;

//         let precise_reward_split = precise_reward_stake_weight
//             .checked_div(&precise_total_reward_stake_weight)
//             .ok_or(TipRouterError::DenominatorIsZero)?;

//         let precise_rewards = precise_rewards_to_process
//             .checked_div(&precise_reward_split)
//             .ok_or(TipRouterError::DenominatorIsZero)?;

//         let floored_precise_rewards = precise_rewards
//             .floor()
//             .ok_or(TipRouterError::ArithmeticFloorError)?;

//         let rewards_u128: u128 = floored_precise_rewards
//             .to_imprecise()
//             .ok_or(TipRouterError::CastToImpreciseNumberError)?;

//         rewards_u128
//             .try_into()
//             .map_err(|_| TipRouterError::CastToU64Error)
//     }

//     pub fn process_reward_pool(
//         &mut self,
//         base_fee: u64,
//         total_reward_stake_weight: u128,
//         reward_stake_weights: &[RewardStakeWeight],
//     ) -> Result<(), TipRouterError> {
//         // Base Rewards
//         {
//             let rewards_to_process: u64 = self.reward_pool();

//             let base_rewards = Self::calculate_base_rewards(base_fee, rewards_to_process)?;

//             self.increment_base_rewards(base_rewards)?;
//             self.decrement_reward_pool(base_rewards)?;
//         }

//         // Router Rewards
//         {
//             let rewards_to_process: u64 = self.reward_pool();

//             for entry in reward_stake_weights.iter() {
//                 let router_rewards = Self::calculate_route_rewards(
//                     entry.reward_stake_weight,
//                     total_reward_stake_weight,
//                     rewards_to_process,
//                 )?;

//                 self.insert_or_increment_router_rewards(entry.destination, router_rewards)?;
//                 self.decrement_reward_pool(router_rewards)?;
//             }
//         }

//         // Any Leftovers go to base
//         {
//             let leftover_rewards = self.reward_pool();

//             self.increment_base_rewards(leftover_rewards)?;
//             self.decrement_reward_pool(leftover_rewards)?;
//         }

//         Ok(())
//     }

//     pub fn insert_or_increment_router_rewards(
//         &mut self,
//         operator: Pubkey,
//         rewards: u64,
//     ) -> Result<(), TipRouterError> {
//         for operator_reward in self.reward_routes.iter_mut() {
//             if operator_reward.destination == operator {
//                 operator_reward.increment_rewards(rewards)?;
//                 return Ok(());
//             }
//         }

//         for operator_reward in self.reward_routes.iter_mut() {
//             if operator_reward.destination == Pubkey::default() {
//                 operator_reward.destination = operator;
//                 operator_reward.rewards = PodU64::from(rewards);
//                 return Ok(());
//             }
//         }

//         Err(TipRouterError::OperatorRewardListFull.into())
//     }

//     pub fn decrement_router_rewards(
//         &mut self,
//         operator: Pubkey,
//         rewards: u64,
//     ) -> Result<(), TipRouterError> {
//         for operator_reward in self.reward_routes.iter_mut() {
//             if operator_reward.destination == operator {
//                 operator_reward.decrement_rewards(rewards)?;
//                 return Ok(());
//             }
//         }

//         Err(TipRouterError::OperatorRewardNotFound.into())
//     }

//     pub fn reward_pool(&self) -> u64 {
//         self.reward_pool.into()
//     }

//     pub fn increment_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.reward_pool = PodU64::from(
//             self.reward_pool()
//                 .checked_add(rewards)
//                 .ok_or(TipRouterError::ArithmeticOverflow)?,
//         );
//         Ok(())
//     }

//     pub fn decrement_reward_pool(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.reward_pool = PodU64::from(
//             self.reward_pool()
//                 .checked_sub(rewards)
//                 .ok_or(TipRouterError::ArithmeticUnderflowError)?,
//         );
//         Ok(())
//     }

//     pub fn base_rewards(&self) -> u64 {
//         self.base_rewards.into()
//     }

//     pub fn increment_base_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.base_rewards = PodU64::from(
//             self.base_rewards()
//                 .checked_add(rewards)
//                 .ok_or(TipRouterError::ArithmeticOverflow)?,
//         );
//         Ok(())
//     }

//     pub fn decrement_base_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.base_rewards = PodU64::from(
//             self.base_rewards()
//                 .checked_sub(rewards)
//                 .ok_or(TipRouterError::ArithmeticUnderflowError)?,
//         );
//         Ok(())
//     }
// }

// #[derive(Debug, Clone, PartialEq, Eq, Copy, Zeroable, ShankType, Pod, ShankType)]
// #[repr(C)]
// pub struct RewardRoutes {
//     destination: Pubkey,
//     rewards: PodU64,
//     reserved: [u8; 128],
// }

// impl Default for RewardRoutes {
//     fn default() -> Self {
//         Self {
//             destination: Pubkey::default(),
//             rewards: PodU64::from(0),
//             reserved: [0; 128],
//         }
//     }
// }

// impl RewardRoutes {
//     pub const fn destination(&self) -> Pubkey {
//         self.destination
//     }

//     pub fn rewards(&self) -> u64 {
//         self.rewards.into()
//     }

//     pub fn increment_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.rewards = PodU64::from(
//             self.rewards()
//                 .checked_add(rewards)
//                 .ok_or(TipRouterError::ArithmeticOverflow)?,
//         );
//         Ok(())
//     }

//     pub fn decrement_rewards(&mut self, rewards: u64) -> Result<(), TipRouterError> {
//         self.rewards = PodU64::from(
//             self.rewards()
//                 .checked_sub(rewards)
//                 .ok_or(TipRouterError::ArithmeticUnderflowError)?,
//         );
//         Ok(())
//     }
// }

// pub struct RewardStakeWeight {
//     pub destination: Pubkey,
//     pub reward_stake_weight: u128,
// }

// impl From<&VaultOperatorStakeWeight> for RewardStakeWeight {
//     fn from(stake_weight: &VaultOperatorStakeWeight) -> Self {
//         Self {
//             destination: stake_weight.vault(),
//             reward_stake_weight: stake_weight.reward_stake_weight(),
//         }
//     }
// }

// impl From<&OperatorVote> for RewardStakeWeight {
//     fn from(vote: &OperatorVote) -> Self {
//         Self {
//             destination: vote.operator(),
//             reward_stake_weight: vote.reward_stake_weight(),
//         }
//     }
// }

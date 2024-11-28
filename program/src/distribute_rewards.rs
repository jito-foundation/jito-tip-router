use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    epoch_reward_router::EpochRewardRouter,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    loaders::load_ncn_epoch,
    ncn_config::NcnConfig,
    operator_epoch_reward_router::OperatorEpochRewardRouter,
    rewards::RewardType,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a Epoch Reward Router
/// Can be backfilled for previous epochs
pub fn process_distribute_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    reward_type: u8,
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, operator, epoch_reward_router, operator_epoch_reward_router, destination, restaking_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, false)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    EpochRewardRouter::load(program_id, ncn.key, ncn_epoch, epoch_reward_router, true)?;
    OperatorEpochRewardRouter::load(
        program_id,
        operator.key,
        ncn.key,
        ncn_epoch,
        operator_epoch_reward_router,
        true,
    )?;

    let reward_type = RewardType::try_from(reward_type)?;

    let rewards = match reward_type {
        RewardType::DAO => {
            process_epoch_reward(&epoch_reward_router)?;
        }
        RewardType::NCN => {
            process_epoch_reward_pool(&epoch_reward_router)?;
        }
        RewardType::OperatorReward => {
            process_operator_epoch_reward_pool(&operator_epoch_reward_router)?;
        }
        RewardType::Vault => {
            process_epoch_reward_pool(&epoch_reward_router)?;
        }
    };

    // Send rewards
    {}

    // let operator_snapshot = {
    //     let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
    //     let operator_snapshot_account =
    //         OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

    //     *operator_snapshot_account
    // };

    // let account_balance = **operator_epoch_reward_router.try_borrow_lamports()?;

    // let mut operator_epoch_reward_router_data =
    //     operator_epoch_reward_router.try_borrow_mut_data()?;
    // let operator_epoch_reward_router_account =
    //     OperatorEpochRewardRouter::try_from_slice_unchecked_mut(
    //         &mut operator_epoch_reward_router_data,
    //     )?;

    // operator_epoch_reward_router_account.process_incoming_rewards(account_balance)?;

    // operator_epoch_reward_router_account.process_reward_pool(&operator_snapshot)?;

    Ok(())
}

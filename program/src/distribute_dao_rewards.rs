use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    base_reward_router::BaseRewardRouter, error::TipRouterError, loaders::load_ncn_epoch,
    ncn_config::NcnConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a Epoch Reward Router
/// Can be backfilled for previous epochs
pub fn process_distribute_dao_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, epoch_reward_router, destination, restaking_program] =
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

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    BaseRewardRouter::load(program_id, ncn.key, ncn_epoch, epoch_reward_router, true)?;

    // Get rewards and update state
    let rewards = {
        let ncn_config_data = ncn_config.try_borrow_data()?;
        let ncn_config_account = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        let fee_config = ncn_config_account.fee_config;

        let mut epoch_reward_router_data = epoch_reward_router.try_borrow_mut_data()?;
        let epoch_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

        epoch_reward_router_account.distribute_dao_rewards(&fee_config, destination.key)?
    };

    // Send rewards
    {
        **destination.lamports.borrow_mut() = destination
            .lamports()
            .checked_add(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        **epoch_reward_router.lamports.borrow_mut() = epoch_reward_router
            .lamports()
            .checked_sub(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
    }

    Ok(())
}

use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    base_reward_router::BaseRewardRouter, error::TipRouterError, loaders::load_ncn_epoch,
    ncn_config::NcnConfig, ncn_fee_group::NcnFeeGroup, ncn_reward_router::NcnRewardRouter,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_distribute_base_ncn_reward_route(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, operator, base_reward_router, ncn_reward_router, restaking_program] =
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
    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    BaseRewardRouter::load(program_id, ncn.key, ncn_epoch, base_reward_router, true)?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        ncn_epoch,
        ncn_reward_router,
        true,
    )?;

    // Get rewards and update state
    let rewards = {
        let mut epoch_reward_router_data = base_reward_router.try_borrow_mut_data()?;
        let base_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

        base_reward_router_account
            .distribute_ncn_fee_group_reward_route(ncn_fee_group, operator.key)?
    };

    // Send rewards
    if rewards > 0 {
        **ncn_reward_router.lamports.borrow_mut() = ncn_reward_router
            .lamports()
            .checked_add(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        **base_reward_router.lamports.borrow_mut() = base_reward_router
            .lamports()
            .checked_sub(rewards)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
    }

    Ok(())
}

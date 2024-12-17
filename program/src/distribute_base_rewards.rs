use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    base_fee_group::BaseFeeGroup, base_reward_router::BaseRewardRouter, error::TipRouterError,
    loaders::load_ncn_epoch, ncn_config::NcnConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_distribute_base_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    base_fee_group: u8,
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, base_reward_router, base_fee_wallet, restaking_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    if !base_fee_wallet.is_writable {
        msg!("Base fee wallet is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;

    BaseRewardRouter::load(program_id, ncn.key, ncn_epoch, base_reward_router, true)?;

    let group = BaseFeeGroup::try_from(base_fee_group)?;

    // Check if base fee wallet is correct
    {
        let ncn_config_data = ncn_config.try_borrow_data()?;
        let ncn_config_account = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        let fee_wallet = ncn_config_account.fee_config.base_fee_wallet(group)?;

        if fee_wallet.ne(base_fee_wallet.key) {
            msg!("Incorrect base fee wallet");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    // Get rewards and update state
    let rewards = {
        let group = BaseFeeGroup::try_from(base_fee_group)?;

        let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
        let base_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

        base_reward_router_account.distribute_base_fee_group_rewards(group)?
    };

    //TODO should this be an error?
    // if rewards == 0 {
    //     msg!("No rewards to distribute");
    //     return Err(TipRouterError::NoRewards.into());
    // }

    // Send rewards
    if rewards > 0 {
        **base_fee_wallet.lamports.borrow_mut() = base_fee_wallet
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

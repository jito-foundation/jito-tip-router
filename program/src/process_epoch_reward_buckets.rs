use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    ballot_box::BallotBox, epoch_reward_router::EpochRewardRouter, loaders::load_ncn_epoch,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a Epoch Reward Router
/// Can be backfilled for previous epochs
pub fn process_process_epoch_reward_buckets(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, ballot_box, epoch_reward_router, restaking_program] = accounts
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

    BallotBox::load(program_id, ncn.key, ncn_epoch, ballot_box, false)?;
    EpochRewardRouter::load(program_id, ncn.key, ncn_epoch, epoch_reward_router, true)?;

    let ballot_box = {
        let ballot_box_data = ballot_box.try_borrow_data()?;
        let ballot_box_account = BallotBox::try_from_slice_unchecked(&ballot_box_data)?;

        *ballot_box_account
    };

    let mut epoch_reward_router_data = epoch_reward_router.try_borrow_mut_data()?;
    let epoch_reward_router_account =
        EpochRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

    epoch_reward_router_account.process_buckets(&ballot_box, program_id)?;

    Ok(())
}

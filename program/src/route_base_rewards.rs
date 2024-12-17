use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    ballot_box::BallotBox, base_reward_router::BaseRewardRouter, epoch_snapshot::EpochSnapshot,
    loaders::load_ncn_epoch,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_route_base_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, epoch_snapshot, ballot_box, base_reward_router, restaking_program] =
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

    EpochSnapshot::load(program_id, ncn.key, ncn_epoch, epoch_snapshot, false)?;
    BaseRewardRouter::load(program_id, ncn.key, ncn_epoch, base_reward_router, true)?;
    BallotBox::load(program_id, ncn.key, ncn_epoch, ballot_box, false)?;

    let epoch_snapshot_data = epoch_snapshot.try_borrow_data()?;
    let epoch_snapshot_account = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

    let ballot_box_data = ballot_box.try_borrow_data()?;
    let ballot_box_account = BallotBox::try_from_slice_unchecked(&ballot_box_data)?;

    let base_reward_router_balance = **base_reward_router.try_borrow_lamports()?;

    let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
    let base_reward_router_account =
        BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

    let rent_cost = base_reward_router_account.rent_cost(&Rent::get()?)?;

    base_reward_router_account.route_incoming_rewards(rent_cost, base_reward_router_balance)?;

    base_reward_router_account.route_reward_pool(epoch_snapshot_account.fees())?;

    base_reward_router_account.route_ncn_fee_group_rewards(ballot_box_account)?;

    Ok(())
}

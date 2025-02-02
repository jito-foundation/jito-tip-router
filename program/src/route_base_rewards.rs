use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as NcnConfig,
    epoch_snapshot::EpochSnapshot,
    epoch_state::EpochState,
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_route_base_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    max_iterations: u16,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, config, ncn, epoch_snapshot, ballot_box, base_reward_router, base_reward_receiver] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    NcnConfig::load(program_id, config, ncn.key, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;

    EpochSnapshot::load(program_id, epoch_snapshot, ncn.key, epoch, false)?;
    BaseRewardRouter::load(program_id, base_reward_router, ncn.key, epoch, true)?;
    BallotBox::load(program_id, ballot_box, ncn.key, epoch, false)?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;

    let epoch_snapshot_data = epoch_snapshot.try_borrow_data()?;
    let epoch_snapshot_account = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

    let ballot_box_data = ballot_box.try_borrow_data()?;
    let ballot_box_account = BallotBox::try_from_slice_unchecked(&ballot_box_data)?;

    let current_slot = Clock::get()?.slot;
    let valid_slots_after_consensus = {
        let ncn_config_data = config.data.borrow();
        let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        ncn_config.valid_slots_after_consensus()
    };

    // Do not route if voting is still ongoing
    if ballot_box_account.is_voting_valid(current_slot, valid_slots_after_consensus)? {
        msg!("Voting is still ongoing, cannot route until voting is complete");
        return Err(TipRouterError::VotingIsNotOver.into());
    }

    let base_reward_receiver_balance = **base_reward_receiver.try_borrow_lamports()?;

    let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
    let base_reward_router_account =
        BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

    let rent_cost = Rent::get()?.minimum_balance(0);

    if !base_reward_router_account.still_routing() {
        base_reward_router_account
            .route_incoming_rewards(rent_cost, base_reward_receiver_balance)?;

        base_reward_router_account.route_reward_pool(epoch_snapshot_account.fees())?;
    }

    base_reward_router_account.route_ncn_fee_group_rewards(ballot_box_account, max_iterations)?;

    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_route_base_rewards(base_reward_router_account.total_rewards());
    }

    Ok(())
}

use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as NcnConfig,
    epoch_state::EpochState,
    error::TipRouterError,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Can be backfilled for previous epochs
pub fn process_distribute_base_ncn_reward_route(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ncn, operator, base_reward_router, base_reward_receiver, ncn_reward_router, ncn_reward_receiver, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    BaseRewardRouter::load(program_id, ncn.key, epoch, base_reward_router, true)?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        ncn_reward_router,
        false,
    )?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;
    NcnRewardReceiver::load(
        program_id,
        ncn_reward_receiver,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        false,
    )?;

    load_system_program(system_program)?;

    // Get rewards and update state
    let rewards = {
        let mut epoch_reward_router_data = base_reward_router.try_borrow_mut_data()?;
        let base_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

        if base_reward_router_account.still_routing() {
            msg!("Rewards still routing");
            return Err(TipRouterError::RouterStillRouting.into());
        }

        base_reward_router_account
            .distribute_ncn_fee_group_reward_route(ncn_fee_group, operator.key)?
    };

    // Send rewards
    if rewards > 0 {
        let (_, base_reward_receiver_bump, mut base_reward_receiver_seeds) =
            BaseRewardReceiver::find_program_address(program_id, ncn.key, epoch);
        base_reward_receiver_seeds.push(vec![base_reward_receiver_bump]);

        solana_program::program::invoke_signed(
            &solana_program::system_instruction::transfer(
                base_reward_receiver.key,
                ncn_reward_receiver.key,
                rewards,
            ),
            &[base_reward_receiver.clone(), ncn_reward_receiver.clone()],
            &[base_reward_receiver_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_distribute_base_ncn_rewards(rewards)?;
    }

    Ok(())
}

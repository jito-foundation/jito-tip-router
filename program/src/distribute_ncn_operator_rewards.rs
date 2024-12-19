use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    error::TipRouterError, ncn_config::NcnConfig, ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::NcnRewardRouter,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Can be backfilled for previous epochs
pub fn process_distribute_ncn_operator_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, operator, ncn_reward_router, ncn_reward_receiver, restaking_program, system_program] =
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
    Operator::load(restaking_program.key, operator, true)?;

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        ncn_reward_router,
        true,
    )?;

    // Get rewards and update state
    let rewards = {
        let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
        let ncn_reward_router_account =
            NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

        ncn_reward_router_account.distribute_operator_rewards()?
    };

    // Send rewards
    if rewards > 0 {
        let (_, ncn_reward_receiver_bump, mut ncn_reward_receiver_seeds) =
            NcnRewardRouter::find_program_address(
                program_id,
                ncn_fee_group,
                operator.key,
                ncn.key,
                epoch,
            );
        ncn_reward_receiver_seeds.push(vec![ncn_reward_receiver_bump]);

        solana_program::program::invoke_signed(
            &solana_program::system_instruction::transfer(
                ncn_reward_receiver.key,
                operator.key,
                rewards,
            ),
            &[ncn_reward_receiver.clone(), operator.clone()],
            &[ncn_reward_receiver_seeds
                .iter()
                .map(|s| s.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
    }

    Ok(())
}

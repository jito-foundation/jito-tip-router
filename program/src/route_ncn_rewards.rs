use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    epoch_snapshot::OperatorSnapshot, ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::NcnRewardRouter,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_route_ncn_rewards(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [restaking_config, ncn, operator, operator_snapshot, ncn_reward_router, restaking_program] =
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

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        epoch,
        operator_snapshot,
        false,
    )?;
    NcnRewardRouter::load(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        ncn_reward_router,
        true,
    )?;

    let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
    let operator_snapshot_account =
        OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

    let account_balance = **ncn_reward_router.try_borrow_lamports()?;

    let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
    let ncn_reward_router_account =
        NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

    let rent_cost = ncn_reward_router_account.rent_cost(&Rent::get()?)?;

    ncn_reward_router_account.route_incoming_rewards(rent_cost, account_balance)?;

    ncn_reward_router_account.route_reward_pool(operator_snapshot_account)?;

    Ok(())
}

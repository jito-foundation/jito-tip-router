use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    epoch_reward_router::EpochRewardRouter, epoch_snapshot::OperatorSnapshot,
    loaders::load_ncn_epoch, operator_epoch_reward_router::OperatorEpochRewardRouter,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Initializes a Epoch Reward Router
/// Can be backfilled for previous epochs
pub fn process_process_operator_epoch_reward_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, operator, operator_snapshot, operator_epoch_reward_router, restaking_program] =
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

    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        ncn_epoch,
        operator_snapshot,
        false,
    )?;
    EpochRewardRouter::load(
        program_id,
        ncn.key,
        ncn_epoch,
        operator_epoch_reward_router,
        true,
    )?;

    let operator_snapshot = {
        let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
        let operator_snapshot_account =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

        operator_snapshot_account.clone()
    };

    let account_balance = operator_epoch_reward_router.try_borrow_lamports()?.clone();

    let mut operator_epoch_reward_router_data =
        operator_epoch_reward_router.try_borrow_mut_data()?;
    let operator_epoch_reward_router_account =
        OperatorEpochRewardRouter::try_from_slice_unchecked_mut(
            &mut operator_epoch_reward_router_data,
        )?;

    operator_epoch_reward_router_account.process_incoming_rewards(account_balance)?;

    operator_epoch_reward_router_account.process_reward_pool(&operator_snapshot)?;

    Ok(())
}

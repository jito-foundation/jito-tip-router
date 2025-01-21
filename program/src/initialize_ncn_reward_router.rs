use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    account_payer::AccountPayer,
    epoch_snapshot::OperatorSnapshot,
    epoch_state::EpochState,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_initialize_ncn_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn, operator, operator_snapshot, ncn_reward_router, ncn_reward_receiver, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    Operator::load(&jito_restaking_program::id(), operator, false)?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        epoch,
        operator_snapshot,
        false,
    )?;
    NcnRewardReceiver::load(
        program_id,
        ncn_reward_receiver,
        ncn_fee_group.try_into()?,
        operator.key,
        ncn.key,
        epoch,
        true,
    )?;

    load_system_account(ncn_reward_router, true)?;
    load_system_program(system_program)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;

    let operator_ncn_index = {
        let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
        let operator_snapshot_account =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;
        operator_snapshot_account.ncn_operator_index()
    };

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    let current_slot = Clock::get()?.slot;

    let (ncn_reward_router_pubkey, ncn_reward_router_bump, mut ncn_reward_router_seeds) =
        NcnRewardRouter::find_program_address(
            program_id,
            ncn_fee_group,
            operator.key,
            ncn.key,
            epoch,
        );
    ncn_reward_router_seeds.push(vec![ncn_reward_router_bump]);

    if ncn_reward_router_pubkey.ne(ncn_reward_router.key) {
        msg!("Incorrect ncn reward router PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Epoch Reward Router {} for NCN: {} at epoch: {}",
        ncn_reward_router.key,
        ncn.key,
        epoch
    );
    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        ncn_reward_router,
        system_program,
        program_id,
        NcnRewardRouter::SIZE,
        &ncn_reward_router_seeds,
    )?;

    let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
    ncn_reward_router_data[0] = NcnRewardRouter::DISCRIMINATOR;
    let ncn_reward_router_account =
        NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

    *ncn_reward_router_account = NcnRewardRouter::new(
        ncn_fee_group,
        operator.key,
        operator_ncn_index,
        ncn.key,
        epoch,
        ncn_reward_router_bump,
        current_slot,
    );

    let min_rent = Rent::get()?.minimum_balance(0);
    msg!(
        "Transferring rent of {} lamports to ncn reward receiver {}",
        min_rent,
        ncn_reward_receiver.key
    );
    AccountPayer::transfer(
        program_id,
        ncn.key,
        account_payer,
        ncn_reward_receiver,
        min_rent,
    )?;

    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account
            .update_realloc_ncn_reward_router(operator_ncn_index as usize, ncn_fee_group)?;
    }

    Ok(())
}

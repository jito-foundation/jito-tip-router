use std::mem::size_of;

use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    epoch_snapshot::OperatorSnapshot,
    epoch_state::EpochState,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction::transfer,
    sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_initialize_ncn_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    ncn_fee_group: u8,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn, operator, operator_snapshot, ncn_reward_router, ncn_reward_receiver, payer, restaking_program, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    EpochState::load(program_id, ncn.key, epoch, epoch_state, true)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, false)?;
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
    load_signer(payer, true)?;

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
    create_account(
        payer,
        ncn_reward_router,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(size_of::<NcnRewardRouter>() as u64)
            .unwrap(),
        &ncn_reward_router_seeds,
    )?;

    let mut ncn_reward_router_data = ncn_reward_router.try_borrow_mut_data()?;
    ncn_reward_router_data[0] = NcnRewardRouter::DISCRIMINATOR;
    let ncn_reward_router_account =
        NcnRewardRouter::try_from_slice_unchecked_mut(&mut ncn_reward_router_data)?;

    *ncn_reward_router_account = NcnRewardRouter::new(
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
        ncn_reward_router_bump,
        current_slot,
    );

    let min_system_account_rent = Rent::get()?.minimum_balance(0);

    msg!(
        "Transferring rent of {} lamports to ncn reward receiver {}",
        min_system_account_rent,
        ncn_reward_receiver.key
    );

    invoke(
        &transfer(payer.key, ncn_reward_receiver.key, min_system_account_rent),
        &[payer.clone(), ncn_reward_receiver.clone()],
    )?;

    {
        let operator_snapshot_data = operator_snapshot.try_borrow_data()?;
        let operator_snapshot_account =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_realloc_ncn_reward_router(
            operator_snapshot_account.ncn_operator_index() as usize,
            ncn_fee_group,
        )?;
    }

    Ok(())
}

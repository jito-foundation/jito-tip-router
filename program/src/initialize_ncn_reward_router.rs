use std::mem::size_of;

use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    loaders::load_ncn_epoch, ncn_fee_group::NcnFeeGroup, ncn_reward_router::NcnRewardRouter,
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
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, operator, ncn_reward_router, payer, restaking_program, system_program] =
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

    load_system_account(ncn_reward_router, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let ncn_fee_group = NcnFeeGroup::try_from(ncn_fee_group)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    let (ncn_reward_router_pubkey, ncn_reward_router_bump, mut ncn_reward_router_seeds) =
        NcnRewardRouter::find_program_address(
            program_id,
            ncn_fee_group,
            operator.key,
            ncn.key,
            ncn_epoch,
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
        ncn_epoch
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
        *operator.key,
        *ncn.key,
        ncn_epoch,
        ncn_reward_router_bump,
        current_slot,
    );

    Ok(())
}

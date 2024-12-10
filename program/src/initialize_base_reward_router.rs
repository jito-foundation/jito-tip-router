use std::mem::size_of;

use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{base_reward_router::BaseRewardRouter, loaders::load_ncn_epoch};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_initialize_base_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, base_reward_router, payer, restaking_program, system_program] =
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

    load_system_account(base_reward_router, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    let (base_reward_router_pubkey, base_reward_router_bump, mut base_reward_router_seeds) =
        BaseRewardRouter::find_program_address(program_id, ncn.key, ncn_epoch);
    base_reward_router_seeds.push(vec![base_reward_router_bump]);

    if base_reward_router_pubkey.ne(base_reward_router.key) {
        msg!("Incorrect epoch reward router PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Base Reward Router {} for NCN: {} at epoch: {}",
        base_reward_router.key,
        ncn.key,
        ncn_epoch
    );
    create_account(
        payer,
        base_reward_router,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(size_of::<BaseRewardRouter>() as u64)
            .unwrap(),
        &base_reward_router_seeds,
    )?;

    let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
    base_reward_router_data[0] = BaseRewardRouter::DISCRIMINATOR;
    let base_reward_router_account =
        BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

    *base_reward_router_account =
        BaseRewardRouter::new(*ncn.key, ncn_epoch, base_reward_router_bump, current_slot);

    Ok(())
}

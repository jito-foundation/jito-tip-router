use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{base_reward_router::BaseRewardRouter, constants::MAX_REALLOC_BYTES};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_initialize_base_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
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

    let (base_reward_router_pubkey, base_reward_router_bump, mut base_reward_router_seeds) =
        BaseRewardRouter::find_program_address(program_id, ncn.key, epoch);
    base_reward_router_seeds.push(vec![base_reward_router_bump]);

    if base_reward_router_pubkey.ne(base_reward_router.key) {
        msg!("Incorrect base reward router PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Base Reward Router {} for NCN: {} at epoch: {}",
        base_reward_router.key,
        ncn.key,
        epoch
    );
    create_account(
        payer,
        base_reward_router,
        system_program,
        program_id,
        &Rent::get()?,
        MAX_REALLOC_BYTES,
        &base_reward_router_seeds,
    )?;

    Ok(())
}

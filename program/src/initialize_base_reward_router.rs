use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    constants::MAX_REALLOC_BYTES,
    epoch_marker::EpochMarker,
    epoch_state::EpochState,
};
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
    let [epoch_marker, epoch_state, ncn, base_reward_router, base_reward_receiver, account_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;
    EpochMarker::check_dne(program_id, ncn.key, epoch, epoch_marker)?;

    load_system_account(base_reward_router, true)?;
    load_system_program(system_program)?;

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
    AccountPayer::pay_and_create_account(
        program_id,
        ncn.key,
        account_payer,
        base_reward_router,
        system_program,
        program_id,
        MAX_REALLOC_BYTES as usize,
        &base_reward_router_seeds,
    )?;

    let min_rent = Rent::get()?.minimum_balance(0);
    msg!(
        "Transferring rent of {} lamports to base reward receiver {}",
        min_rent,
        base_reward_receiver.key
    );
    AccountPayer::transfer(
        program_id,
        ncn.key,
        account_payer,
        base_reward_receiver,
        min_rent,
    )?;

    Ok(())
}

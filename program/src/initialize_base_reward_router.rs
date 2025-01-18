use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    constants::MAX_REALLOC_BYTES,
    epoch_state::EpochState,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction::transfer,
    sysvar::Sysvar,
};

/// Can be backfilled for previous epochs
pub fn process_initialize_base_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn, base_reward_router, base_reward_receiver, payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;

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

    let min_system_account_rent = Rent::get()?.minimum_balance(0);

    msg!(
        "Transferring rent of {} lamports to base reward receiver {}",
        min_system_account_rent,
        base_reward_receiver.key
    );

    invoke(
        &transfer(payer.key, base_reward_receiver.key, min_system_account_rent),
        &[
            payer.clone(),
            base_reward_receiver.clone(),
            system_program.clone(),
        ],
    )?;

    Ok(())
}

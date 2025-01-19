use jito_jsm_core::loader::{load_system_account, load_system_program};
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    claim_status_payer::ClaimStatusPayer,
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
    let [epoch_state, ncn, base_reward_router, base_reward_receiver, claim_status_payer, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, ncn.key, epoch, epoch_state, false)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    BaseRewardReceiver::load(program_id, base_reward_receiver, ncn.key, epoch, true)?;
    ClaimStatusPayer::load(program_id, claim_status_payer, true)?;

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
    ClaimStatusPayer::pay_and_create_account(
        program_id,
        claim_status_payer,
        base_reward_router,
        system_program,
        program_id,
        BaseRewardRouter::SIZE,
        &base_reward_router_seeds,
    )?;

    let min_rent = Rent::get()?.minimum_balance(0);
    msg!(
        "Transferring rent of {} lamports to base reward receiver {}",
        min_rent,
        base_reward_receiver.key
    );
    ClaimStatusPayer::transfer(
        program_id,
        claim_status_payer,
        base_reward_receiver,
        min_rent,
    )?;

    Ok(())
}

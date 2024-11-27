use std::mem::size_of;

use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    create_account,
    loader::{load_signer, load_system_account, load_system_program},
};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    ballot_box::BallotBox, epoch_reward_router::EpochRewardRouter, loaders::load_ncn_epoch,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

/// Initializes a Epoch Reward Router
/// Can be backfilled for previous epochs
pub fn process_initialize_epoch_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    first_slot_of_ncn_epoch: Option<u64>,
) -> ProgramResult {
    let [restaking_config, ncn, ballot_box, epoch_reward_router, payer, restaking_program, system_program] =
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

    load_system_account(epoch_reward_router, true)?;
    load_system_program(system_program)?;
    load_signer(payer, true)?;

    let current_slot = Clock::get()?.slot;
    let (ncn_epoch, _) = load_ncn_epoch(restaking_config, current_slot, first_slot_of_ncn_epoch)?;

    let has_winning_ballot = {
        let ballot_box_data = ballot_box.data.borrow();
        let ballot_box = BallotBox::try_from_slice_unchecked(&ballot_box_data)?;
        ballot_box.has_winning_ballot()
    };

    if !has_winning_ballot {
        msg!("Ballot has to be finalized before initializing epoch reward router");
        return Err(ProgramError::InvalidAccountData);
    }

    let (epoch_reward_router_pubkey, epoch_reward_router_bump, mut epoch_reward_router_seeds) =
        EpochRewardRouter::find_program_address(program_id, ncn.key, ncn_epoch);
    epoch_reward_router_seeds.push(vec![epoch_reward_router_bump]);

    if epoch_reward_router_pubkey.ne(epoch_reward_router.key) {
        msg!("Incorrect epoch reward router PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    msg!(
        "Initializing Epoch Reward Router {} for NCN: {} at epoch: {}",
        epoch_reward_router.key,
        ncn.key,
        ncn_epoch
    );
    create_account(
        payer,
        epoch_reward_router,
        system_program,
        program_id,
        &Rent::get()?,
        8_u64
            .checked_add(size_of::<EpochRewardRouter>() as u64)
            .unwrap(),
        &epoch_reward_router_seeds,
    )?;

    let mut epoch_reward_router_data = epoch_reward_router.try_borrow_mut_data()?;
    epoch_reward_router_data[0] = EpochRewardRouter::DISCRIMINATOR;
    let epoch_reward_router_account =
        EpochRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

    *epoch_reward_router_account =
        EpochRewardRouter::new(*ncn.key, ncn_epoch, epoch_reward_router_bump, current_slot);

    Ok(())
}

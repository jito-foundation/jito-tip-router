use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    loader::{load_signer, load_system_program},
    realloc,
};
use jito_tip_router_core::{
    base_reward_router::BaseRewardRouter, config::Config as NcnConfig, utils::get_new_size,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_realloc_base_reward_router(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, base_reward_router, ncn, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    load_signer(payer, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;

    let (base_reward_router_pda, base_reward_router_bump, _) =
        BaseRewardRouter::find_program_address(program_id, ncn.key, epoch);

    if base_reward_router_pda != *base_reward_router.key {
        msg!("Base reward router account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if base_reward_router.data_len() < BaseRewardRouter::SIZE {
        let new_size = get_new_size(base_reward_router.data_len(), BaseRewardRouter::SIZE)?;
        msg!(
            "Reallocating base reward router from {} bytes to {} bytes",
            base_reward_router.data_len(),
            new_size
        );
        realloc(base_reward_router, new_size, payer, &Rent::get()?)?;
    }

    let should_initialize = base_reward_router.data_len() >= BaseRewardRouter::SIZE
        && base_reward_router.try_borrow_data()?[0] != BaseRewardRouter::DISCRIMINATOR;

    if should_initialize {
        let mut base_reward_router_data = base_reward_router.try_borrow_mut_data()?;
        base_reward_router_data[0] = BaseRewardRouter::DISCRIMINATOR;
        let base_reward_router_account =
            BaseRewardRouter::try_from_slice_unchecked_mut(&mut base_reward_router_data)?;

        base_reward_router_account.initialize(
            ncn.key,
            epoch,
            base_reward_router_bump,
            Clock::get()?.slot,
        );
    }

    Ok(())
}

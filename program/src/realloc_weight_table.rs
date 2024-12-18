use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    loader::{load_signer, load_system_program},
    realloc,
};
use jito_tip_router_core::{
    ncn_config::NcnConfig, tracked_mints::TrackedMints, utils::get_new_size,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, sysvar::Sysvar,
};

pub fn process_realloc_weight_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, weight_table, ncn, tracked_mints, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    load_signer(payer, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    TrackedMints::load(program_id, ncn.key, tracked_mints, false)?;

    let (weight_table_pda, weight_table_bump, _) =
        WeightTable::find_program_address(program_id, ncn.key, epoch);

    if weight_table_pda != *weight_table.key {
        msg!("Weight table account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if weight_table.data_len() < WeightTable::SIZE {
        let new_size = get_new_size(weight_table.data_len(), WeightTable::SIZE)?;
        msg!(
            "Reallocating weight table from {} bytes to {} bytes",
            weight_table.data_len(),
            new_size
        );
        realloc(weight_table, new_size, payer, &Rent::get()?)?;
    }

    let should_initialize = weight_table.data_len() >= WeightTable::SIZE
        && weight_table.try_borrow_data()?[0] != WeightTable::DISCRIMINATOR;

    if should_initialize {
        let unique_mints = {
            let tracked_mints_data = tracked_mints.data.borrow();
            let tracked_mints = TrackedMints::try_from_slice_unchecked(&tracked_mints_data)?;
            tracked_mints.get_unique_mints()
        };

        let mut weight_table_data = weight_table.try_borrow_mut_data()?;
        weight_table_data[0] = WeightTable::DISCRIMINATOR;
        let weight_table_account =
            WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

        weight_table_account.initialize(
            *ncn.key,
            epoch,
            Clock::get()?.slot,
            weight_table_bump,
            &unique_mints,
        )?;
    }

    Ok(())
}

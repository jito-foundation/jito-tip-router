use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::{
    loader::{load_signer, load_system_program},
    realloc,
};
use jito_tip_router_core::{
    ncn_config::NcnConfig, utils::get_new_size, vault_registry::VaultRegistry,
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
    let [ncn_config, weight_table, ncn, vault_registry, payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    load_system_program(system_program)?;
    load_signer(payer, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    VaultRegistry::load(program_id, ncn.key, vault_registry, false)?;

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
        let vault_registry_data = vault_registry.data.borrow();
        let vault_registry = VaultRegistry::try_from_slice_unchecked(&vault_registry_data)?;

        let vault_count = vault_registry.vault_count();
        let vault_entries = vault_registry.get_vault_entries();
        let mint_entries = vault_registry.get_mint_entries();

        let mut weight_table_data = weight_table.try_borrow_mut_data()?;
        weight_table_data[0] = WeightTable::DISCRIMINATOR;
        let weight_table_account =
            WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

        weight_table_account.initialize(
            *ncn.key,
            epoch,
            Clock::get()?.slot,
            vault_count,
            weight_table_bump,
            vault_entries,
            mint_entries,
        )?;
    }

    Ok(())
}

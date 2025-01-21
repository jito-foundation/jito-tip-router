use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_jsm_core::loader::load_system_program;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    account_payer::AccountPayer, config::Config as NcnConfig, utils::get_new_size,
    vault_registry::VaultRegistry,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_realloc_vault_registry(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let [ncn_config, vault_registry, ncn, account_payer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify accounts
    load_system_program(system_program)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    AccountPayer::load(program_id, ncn.key, account_payer, true)?;

    let (vault_registry_pda, vault_registry_bump, mut vault_registry_seeds) =
        VaultRegistry::find_program_address(program_id, ncn.key);
    vault_registry_seeds.push(vec![vault_registry_bump]);

    if vault_registry_pda != *vault_registry.key {
        return Err(ProgramError::InvalidSeeds);
    }

    if vault_registry.data_len() < VaultRegistry::SIZE {
        let new_size = get_new_size(vault_registry.data_len(), VaultRegistry::SIZE)?;
        msg!(
            "Reallocating vault registry from {} bytes to {} bytes",
            vault_registry.data_len(),
            new_size
        );

        AccountPayer::pay_and_realloc(
            program_id,
            ncn.key,
            account_payer,
            vault_registry,
            new_size,
        )?;
    }

    let should_initialize = vault_registry.data_len() >= VaultRegistry::SIZE
        && vault_registry.try_borrow_data()?[0] != VaultRegistry::DISCRIMINATOR;

    if should_initialize {
        let mut vault_registry_data = vault_registry.try_borrow_mut_data()?;
        vault_registry_data[0] = VaultRegistry::DISCRIMINATOR;
        let vault_registry_account =
            VaultRegistry::try_from_slice_unchecked_mut(&mut vault_registry_data)?;
        vault_registry_account.initialize(ncn.key, vault_registry_bump);
    }

    Ok(())
}

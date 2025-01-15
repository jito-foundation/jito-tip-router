use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{ncn::Ncn, ncn_vault_ticket::NcnVaultTicket};
use jito_tip_router_core::vault_registry::VaultRegistry;
use jito_vault_core::vault::Vault;
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::{clock::Clock, Sysvar},
};

pub fn process_register_vault(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [vault_registry, ncn, vault, ncn_vault_ticket, restaking_program_id, vault_program_id] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    VaultRegistry::load(program_id, ncn.key, vault_registry, true)?;
    Ncn::load(restaking_program_id.key, ncn, false)?;
    Vault::load(vault_program_id.key, vault, false)?;
    NcnVaultTicket::load(
        restaking_program_id.key,
        ncn_vault_ticket,
        ncn,
        vault,
        false,
    )?;

    let clock = Clock::get()?;
    let slot = clock.slot;

    let mut vault_registry_data = vault_registry.try_borrow_mut_data()?;
    let vault_registry = VaultRegistry::try_from_slice_unchecked_mut(&mut vault_registry_data)?;

    let vault_data = vault.data.borrow();
    let vault_account = Vault::try_from_slice_unchecked(&vault_data)?;

    if !vault_registry.has_st_mint(&vault_account.supported_mint) {
        msg!("Supported mint not registered");
        return Err(ProgramError::InvalidAccountData);
    }

    vault_registry.register_vault(
        vault.key,
        &vault_account.supported_mint,
        vault_account.vault_index(),
        slot,
    )?;

    Ok(())
}

use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{ncn_config::NcnConfig, vault_registry::VaultRegistry};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

pub fn process_admin_set_st_mint(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    st_mint: Pubkey,
    ncn_fee_group: Option<u8>,
    reward_multiplier_bps: Option<u64>,
    switchboard_feed: Option<Pubkey>,
    no_feed_weight: Option<u128>,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, vault_registry, admin, restaking_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    VaultRegistry::load(program_id, ncn.key, vault_registry, true)?;
    Config::load(restaking_program.key, restaking_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;

    load_signer(admin, false)?;

    {
        let ncn_data = ncn.data.borrow();
        let ncn_account = Ncn::try_from_slice_unchecked(&ncn_data)?;

        if ncn_account.ncn_program_admin.ne(admin.key) {
            msg!("Admin is not the NCN program admin");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let mut vault_registry_data = vault_registry.data.borrow_mut();
    let vault_registry_account =
        VaultRegistry::try_from_slice_unchecked_mut(&mut vault_registry_data)?;

    vault_registry_account.set_st_mint(
        &st_mint,
        ncn_fee_group,
        reward_multiplier_bps,
        switchboard_feed,
        no_feed_weight,
    )?;

    Ok(())
}
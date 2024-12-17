use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::{load_signer, load_system_account};
use jito_restaking_core::{config::Config, ncn::Ncn};
use jito_tip_router_core::{
    error::TipRouterError, ncn_config::NcnConfig, ncn_fee_group, tracked_mints::TrackedMints,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_program,
    sysvar::{clock::Clock, Sysvar},
};

pub fn process_set_tracked_mint_ncn_fee_group(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    vault_index: u64,
    ncn_fee_group: u8,
) -> ProgramResult {
    let [restaking_config, ncn_config, ncn, weight_table, tracked_mints, admin, restaking_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    TrackedMints::load(program_id, ncn.key, tracked_mints, true)?;
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

    let epoch = Clock::get()?.epoch;

    // Once tracked_mints.mint_count() == ncn.vault_count, the weight table can be initialized
    // Once the weight table is initialized, you can't add any more mints
    if weight_table.owner.eq(&system_program::ID) {
        let expected_pubkey = WeightTable::find_program_address(program_id, ncn.key, epoch).0;
        if weight_table.key.ne(&expected_pubkey) {
            msg!("Weight table incorrect PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        load_system_account(weight_table, false)?;
    }

    if weight_table.owner.eq(program_id) {
        WeightTable::load(program_id, weight_table, ncn, epoch, false)?;
        return Err(TipRouterError::TrackedMintsLocked.into());
    }

    let mut tracked_mints_data = tracked_mints.data.borrow_mut();
    let tracked_mints_account =
        TrackedMints::try_from_slice_unchecked_mut(&mut tracked_mints_data)?;

    let ncn_fee_group = ncn_fee_group::NcnFeeGroup::try_from(ncn_fee_group)?;
    tracked_mints_account.set_ncn_fee_group(vault_index, ncn_fee_group)?;

    Ok(())
}

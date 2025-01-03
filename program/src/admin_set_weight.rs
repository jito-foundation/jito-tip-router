use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{error::TipRouterError, weight_table::WeightTable};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Updates weight table
pub fn process_admin_set_weight(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    st_mint: &Pubkey,
    epoch: u64,
    weight: u128,
) -> ProgramResult {
    let [ncn, weight_table, weight_table_admin, restaking_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(restaking_program.key, ncn, false)?;
    let ncn_weight_table_admin = {
        let ncn_data = ncn.data.borrow();
        let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn.weight_table_admin
    };

    load_signer(weight_table_admin, true)?;
    WeightTable::load(program_id, weight_table, ncn.key, epoch, true)?;

    if restaking_program.key.ne(&jito_restaking_program::id()) {
        msg!("Incorrect restaking program ID");
        return Err(ProgramError::InvalidAccountData);
    }

    if ncn_weight_table_admin.ne(weight_table_admin.key) {
        msg!("Vault update delegations ticket is not at the correct PDA");
        return Err(TipRouterError::IncorrectWeightTableAdmin.into());
    }

    let mut weight_table_data = weight_table.try_borrow_mut_data()?;
    let weight_table_account = WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

    weight_table_account.check_table_initialized()?;
    if weight_table_account.finalized() {
        msg!("Weight table is finalized");
        return Err(ProgramError::InvalidAccountData);
    }

    weight_table_account.set_weight(st_mint, weight, Clock::get()?.slot)?;

    Ok(())
}

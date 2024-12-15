use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_token_mint;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    constants::{DEFAULT_LST_WEIGHT, WEIGHT_PRECISION},
    error::TipRouterError,
    weight_table::WeightTable,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

/// Updates weight table
pub fn process_set_lst_weight(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    epoch: u64,
) -> ProgramResult {
    let [ncn, weight_table, mint] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;

    load_token_mint(mint)?;
    WeightTable::load(program_id, weight_table, ncn, epoch, true)?;

    let mut weight_table_data = weight_table.try_borrow_mut_data()?;
    let weight_table_account = WeightTable::try_from_slice_unchecked_mut(&mut weight_table_data)?;

    weight_table_account.check_initialized()?;
    if weight_table_account.finalized() {
        msg!("Weight table is finalized");
        return Err(ProgramError::InvalidAccountData);
    }

    let weight = {
        DEFAULT_LST_WEIGHT
            .checked_mul(WEIGHT_PRECISION)
            .ok_or(TipRouterError::ArithmeticOverflow)?
    };

    weight_table_account.set_weight(mint.key, weight, Clock::get()?.slot)?;

    Ok(())
}

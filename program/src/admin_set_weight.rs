use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    epoch_state::EpochState, error::TipRouterError, weight_table::WeightTable,
};
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
    let [epoch_state, ncn, weight_table, weight_table_admin] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    let ncn_weight_table_admin = {
        let ncn_data = ncn.data.borrow();
        let ncn = Ncn::try_from_slice_unchecked(&ncn_data)?;
        ncn.weight_table_admin
    };

    load_signer(weight_table_admin, true)?;
    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    WeightTable::load(program_id, weight_table, ncn.key, epoch, true)?;

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

    // Update Epoch State
    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_set_weight(
            weight_table_account.weight_count() as u64,
            weight_table_account.st_mint_count() as u64,
        );
    }

    Ok(())
}

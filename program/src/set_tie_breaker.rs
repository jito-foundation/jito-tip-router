use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{ballot_box::BallotBox, error::TipRouterError, ncn_config::NcnConfig};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_set_tie_breaker(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    meta_merkle_root: [u8; 32],
    ncn_epoch: u64,
) -> ProgramResult {
    let [ncn_config, ballot_box, ncn, tie_breaker_admin, restaking_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    BallotBox::load(program_id, ncn.key, ncn_epoch, ballot_box, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    load_signer(tie_breaker_admin, false)?;

    let ncn_config_data = ncn_config.data.borrow();
    let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;

    if ncn_config.tie_breaker_admin.ne(tie_breaker_admin.key) {
        msg!("Tie breaker admin invalid");
        return Err(TipRouterError::TieBreakerAdminInvalid.into());
    }

    let mut ballot_box_data = ballot_box.data.borrow_mut();
    let ballot_box_account = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;

    let current_epoch = Clock::get()?.epoch;

    ballot_box_account.set_tie_breaker_ballot(
        meta_merkle_root,
        current_epoch,
        ncn_config.epochs_before_stall(),
    )?;

    Ok(())
}

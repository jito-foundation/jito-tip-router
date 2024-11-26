use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::ncn::Ncn;
use jito_tip_router_core::{
    ballot_box::{Ballot, BallotBox},
    error::TipRouterError,
    ncn_config::NcnConfig,
};
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
    // accounts: [ncn_config, ballot_box, ncn, tie_breaker_admin(signer)]

    let [ncn_config, ballot_box, ncn, tie_breaker_admin] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    BallotBox::load(program_id, ncn.key, ncn_epoch, ballot_box, false)?;
    Ncn::load(program_id, ncn, false)?;

    load_signer(tie_breaker_admin, false)?;

    let ncn_config_data = ncn_config.data.borrow();
    let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;

    if ncn_config.tie_breaker_admin.ne(tie_breaker_admin.key) {
        msg!("Tie breaker admin invalid");
        return Err(TipRouterError::TieBreakerAdminInvalid.into());
    }

    let mut ballot_box_data = ballot_box.data.borrow_mut();
    let ballot_box_account = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;

    // Check that consensus has not been reached and we are past epoch
    if ballot_box_account.is_consensus_reached() {
        msg!("Consensus already reached");
        return Err(TipRouterError::ConsensusAlreadyReached.into());
    }

    let current_epoch = Clock::get()?.epoch;

    // Check if voting is stalled and setting the tie breaker is eligible
    if ballot_box_account.epoch() + ncn_config.epochs_before_stall() < current_epoch {
        return Err(TipRouterError::VotingNotFinalized.into());
    }

    let finalized_ballot = Ballot::new(meta_merkle_root);

    // Check that the merkle root is one of the existing options
    if !ballot_box_account.has_ballot(&finalized_ballot) {
        return Err(TipRouterError::TieBreakerNotInPriorVotes.into());
    }

    ballot_box_account.set_winning_ballot(finalized_ballot);

    Ok(())
}

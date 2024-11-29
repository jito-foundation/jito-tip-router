use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    ballot_box::{Ballot, BallotBox},
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    error::TipRouterError,
    ncn_config::NcnConfig,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    meta_merkle_root: [u8; 32],
    ncn_epoch: u64,
) -> ProgramResult {
    let [ncn_config, ballot_box, ncn, epoch_snapshot, operator_snapshot, operator] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Operator is casting the vote, needs to be signer
    load_signer(operator, false)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Ncn::load(program_id, ncn, false)?;
    Operator::load(program_id, operator, false)?;

    BallotBox::load(program_id, ncn.key, ncn_epoch, ballot_box, true)?;
    EpochSnapshot::load(
        program_id,
        epoch_snapshot.key,
        ncn_epoch,
        epoch_snapshot,
        false,
    )?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        ncn_epoch,
        operator_snapshot,
        false,
    )?;

    let valid_slots_after_consensus = {
        let ncn_config_data = ncn_config.data.borrow();
        let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        ncn_config.valid_slots_after_consensus()
    };

    let mut ballot_box_data = ballot_box.data.borrow_mut();
    let ballot_box = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;

    let total_stake_weight = {
        let epoch_snapshot_data = epoch_snapshot.data.borrow();
        let epoch_snapshot = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

        if !epoch_snapshot.finalized() {
            return Err(TipRouterError::EpochSnapshotNotFinalized.into());
        }

        epoch_snapshot.stake_weight()
    };

    let operator_stake_weight = {
        let operator_snapshot_data = operator_snapshot.data.borrow();
        let operator_snapshot =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

        operator_snapshot.stake_weight()
    };

    let slot = Clock::get()?.slot;

    let ballot = Ballot::new(meta_merkle_root);

    ballot_box.cast_vote(
        *operator.key,
        ballot,
        operator_stake_weight,
        slot,
        valid_slots_after_consensus,
    )?;

    ballot_box.tally_votes(total_stake_weight, slot)?;

    if ballot_box.is_consensus_reached() {
        msg!(
            "Consensus reached for epoch {} with ballot {}",
            ncn_epoch,
            ballot_box.get_winning_ballot()?
        );
    }

    Ok(())
}

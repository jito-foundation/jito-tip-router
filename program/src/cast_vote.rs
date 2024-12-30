use jito_bytemuck::AccountDeserialize;
use jito_jsm_core::loader::load_signer;
use jito_restaking_core::{ncn::Ncn, operator::Operator};
use jito_tip_router_core::{
    ballot_box::{Ballot, BallotBox},
    config::Config as NcnConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, clock::Clock, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey, sysvar::Sysvar,
};

pub fn process_cast_vote(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    meta_merkle_root: &[u8; 32],
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, ballot_box, ncn, epoch_snapshot, operator_snapshot, operator, operator_admin, restaking_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Operator is casting the vote, needs to be signer
    load_signer(operator_admin, false)?;

    NcnConfig::load(program_id, ncn.key, ncn_config, false)?;
    Ncn::load(restaking_program.key, ncn, false)?;
    Operator::load(restaking_program.key, operator, false)?;

    BallotBox::load(program_id, ncn.key, epoch, ballot_box, true)?;
    EpochSnapshot::load(program_id, ncn.key, epoch, epoch_snapshot, false)?;
    OperatorSnapshot::load(
        program_id,
        operator.key,
        ncn.key,
        epoch,
        operator_snapshot,
        false,
    )?;
    let operator_data = operator.data.borrow();
    let operator_account = Operator::try_from_slice_unchecked(&operator_data)?;

    if *operator_admin.key != operator_account.admin {
        return Err(TipRouterError::OperatorAdminInvalid.into());
    }

    let valid_slots_after_consensus = {
        let ncn_config_data = ncn_config.data.borrow();
        let ncn_config = NcnConfig::try_from_slice_unchecked(&ncn_config_data)?;
        ncn_config.valid_slots_after_consensus()
    };

    let mut ballot_box_data = ballot_box.data.borrow_mut();
    let ballot_box = BallotBox::try_from_slice_unchecked_mut(&mut ballot_box_data)?;

    let total_stake_weights = {
        let epoch_snapshot_data = epoch_snapshot.data.borrow();
        let epoch_snapshot = EpochSnapshot::try_from_slice_unchecked(&epoch_snapshot_data)?;

        if !epoch_snapshot.finalized() {
            return Err(TipRouterError::EpochSnapshotNotFinalized.into());
        }

        *epoch_snapshot.stake_weights()
    };

    let operator_stake_weights = {
        let operator_snapshot_data = operator_snapshot.data.borrow();
        let operator_snapshot =
            OperatorSnapshot::try_from_slice_unchecked(&operator_snapshot_data)?;

        *operator_snapshot.stake_weights()
    };

    let slot = Clock::get()?.slot;

    let ballot = Ballot::new(meta_merkle_root);

    ballot_box.cast_vote(
        operator.key,
        &ballot,
        &operator_stake_weights,
        slot,
        valid_slots_after_consensus,
    )?;

    ballot_box.tally_votes(total_stake_weights.stake_weight(), slot)?;

    if ballot_box.is_consensus_reached() {
        msg!(
            "Consensus reached for epoch {} with ballot {:?}",
            epoch,
            ballot_box.get_winning_ballot_tally()?
        );
    }

    Ok(())
}

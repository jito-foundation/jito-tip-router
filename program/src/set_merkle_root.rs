use jito_bytemuck::AccountDeserialize;
use jito_priority_fee_distribution_sdk::{
    derive_priority_fee_distribution_account_address,
    instruction::upload_merkle_root_ix as pf_upload_merkle_root_ix, jito_priority_fee_distribution,
};
use jito_restaking_core::ncn::Ncn;
use jito_tip_distribution_sdk::{
    derive_tip_distribution_account_address, instruction::upload_merkle_root_ix,
    jito_tip_distribution,
};
use jito_tip_router_core::{
    ballot_box::BallotBox, config::Config as NcnConfig, epoch_state::EpochState,
    error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

#[allow(clippy::too_many_arguments)]
pub fn process_set_merkle_root(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    merkle_root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
    epoch: u64,
) -> ProgramResult {
    let [epoch_state, ncn_config, ncn, ballot_box, vote_account, distribution_account, distribution_config, distribution_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    EpochState::load(program_id, epoch_state, ncn.key, epoch, true)?;
    NcnConfig::load(program_id, ncn_config, ncn.key, true)?;
    Ncn::load(&jito_restaking_program::id(), ncn, false)?;
    BallotBox::load(program_id, ballot_box, ncn.key, epoch, false)?;

    let distribution_program_id = distribution_program.key;
    if [
        jito_tip_distribution::ID,
        jito_priority_fee_distribution::ID,
    ]
    .iter()
    .all(|supported_program_id| distribution_program_id.ne(supported_program_id))
    {
        msg!("Incorrect tip distribution program");
        return Err(ProgramError::InvalidAccountData);
    }

    let distribution_epoch = epoch
        .checked_sub(1)
        .ok_or(TipRouterError::ArithmeticUnderflowError)?;
    let (distribution_account_address, _) =
        if distribution_program_id.eq(&jito_tip_distribution::ID) {
            derive_tip_distribution_account_address(
                distribution_program.key,
                vote_account.key,
                distribution_epoch,
            )
        } else {
            derive_priority_fee_distribution_account_address(
                distribution_program.key,
                vote_account.key,
                distribution_epoch,
            )
        };
    if distribution_account_address.ne(distribution_account.key) {
        msg!("Incorrect tip distribution account");
        return Err(ProgramError::InvalidAccountData);
    }

    let ballot_box_data = ballot_box.data.borrow();
    let ballot_box = BallotBox::try_from_slice_unchecked(&ballot_box_data)?;

    if !ballot_box.is_consensus_reached() {
        msg!("Ballot box not finalized");
        return Err(TipRouterError::ConsensusNotReached.into());
    }

    ballot_box.verify_merkle_root(
        &distribution_account_address,
        proof,
        &merkle_root,
        max_total_claim,
        max_num_nodes,
    )?;

    let (_, bump, mut ncn_config_seeds) = NcnConfig::find_program_address(program_id, ncn.key);
    ncn_config_seeds.push(vec![bump]);

    let ix = if distribution_program_id.eq(&jito_tip_distribution::ID) {
        upload_merkle_root_ix(
            *distribution_config.key,
            *ncn_config.key,
            *distribution_account.key,
            merkle_root,
            max_total_claim,
            max_num_nodes,
        )
    } else {
        pf_upload_merkle_root_ix(
            *distribution_config.key,
            *ncn_config.key,
            *distribution_account.key,
            merkle_root,
            max_total_claim,
            max_num_nodes,
        )
    };

    invoke_signed(
        &ix,
        &[
            distribution_config.clone(),
            distribution_account.clone(),
            ncn_config.clone(),
        ],
        &[ncn_config_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;

    // Update Epoch State
    {
        let mut epoch_state_data = epoch_state.try_borrow_mut_data()?;
        let epoch_state_account = EpochState::try_from_slice_unchecked_mut(&mut epoch_state_data)?;
        epoch_state_account.update_set_merkle_root()?;
    }

    Ok(())
}

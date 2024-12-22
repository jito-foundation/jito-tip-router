use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::ncn::Ncn;
use jito_tip_distribution_sdk::{
    derive_tip_distribution_account_address, instruction::upload_merkle_root_ix,
};
use jito_tip_router_core::{
    ballot_box::BallotBox, config::Config as NcnConfig, error::TipRouterError,
};
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

pub fn process_set_merkle_root(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    merkle_root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
    epoch: u64,
) -> ProgramResult {
    let [ncn_config, ncn, ballot_box, vote_account, tip_distribution_account, tip_distribution_config, tip_distribution_program_id, restaking_program_id] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    NcnConfig::load(program_id, ncn.key, ncn_config, true)?;
    Ncn::load(restaking_program_id.key, ncn, false)?;
    BallotBox::load(program_id, ncn.key, epoch, ballot_box, false)?;

    let (tip_distribution_address, _) = derive_tip_distribution_account_address(
        tip_distribution_program_id.key,
        vote_account.key,
        epoch,
    );

    if tip_distribution_address.ne(tip_distribution_account.key) {
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
        &tip_distribution_address,
        proof,
        &merkle_root,
        max_total_claim,
        max_num_nodes,
    )?;

    let (_, bump, mut ncn_config_seeds) = NcnConfig::find_program_address(program_id, ncn.key);
    ncn_config_seeds.push(vec![bump]);

    invoke_signed(
        &upload_merkle_root_ix(
            *tip_distribution_config.key,
            *ncn_config.key,
            *tip_distribution_account.key,
            merkle_root,
            max_total_claim,
            max_num_nodes,
        ),
        &[
            tip_distribution_config.clone(),
            tip_distribution_account.clone(),
            ncn_config.clone(),
        ],
        &[ncn_config_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;

    Ok(())
}

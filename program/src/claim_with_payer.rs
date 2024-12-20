use jito_tip_distribution_sdk::instruction::claim_ix;
use jito_tip_router_core::claim_status_payer::ClaimStatusPayer;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey,
};

pub fn process_claim_with_payer(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> ProgramResult {
    let [claim_status_payer, tip_distribution_program, config, tip_distribution_account, claim_status, claimant, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify claim status address
    ClaimStatusPayer::load(
        program_id,
        claim_status_payer,
        tip_distribution_program.key,
        true,
    )?;

    let (_, claim_status_payer_bump, mut claim_status_payer_seeds) =
        ClaimStatusPayer::find_program_address(program_id, tip_distribution_program.key);
    claim_status_payer_seeds.push(vec![claim_status_payer_bump]);

    // Invoke the claim instruction with our program as the payer
    invoke_signed(
        &claim_ix(
            *config.key,
            *tip_distribution_account.key,
            *claim_status.key,
            *claimant.key,
            *claim_status_payer.key,
            *system_program.key,
            proof,
            amount,
            bump,
        ),
        &[
            config.clone(),
            tip_distribution_account.clone(),
            claim_status.clone(),
            claimant.clone(),
            claim_status_payer.clone(),
            system_program.clone(),
        ],
        &[claim_status_payer_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;

    Ok(())
}

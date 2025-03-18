use jito_priority_fee_distribution_sdk::jito_priority_fee_distribution;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::claim_with_payer::_process_claim_with_payer;

pub fn process_claim_with_payer_priority_fee(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> ProgramResult {
    _process_claim_with_payer(
        program_id,
        &jito_priority_fee_distribution::ID,
        accounts,
        proof,
        amount,
        bump,
    )
}

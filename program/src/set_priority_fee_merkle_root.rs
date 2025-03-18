use jito_priority_fee_distribution_sdk::jito_priority_fee_distribution;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey};

use crate::set_merkle_root::_process_set_merkle_root;

pub fn process_set_priority_fee_merkle_root(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    proof: Vec<[u8; 32]>,
    merkle_root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
    epoch: u64,
) -> ProgramResult {
    _process_set_merkle_root(
        program_id,
        &jito_priority_fee_distribution::ID,
        accounts,
        proof,
        merkle_root,
        max_total_claim,
        max_num_nodes,
        epoch,
    )
}

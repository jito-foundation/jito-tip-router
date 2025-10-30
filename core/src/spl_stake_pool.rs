//! Minimal SPL Stake Pool utilities
//!
//! This module contains only the minimal code needed to interact with the SPL Stake Pool program,
//! specifically for depositing SOL. This avoids depending on the full spl-stake-pool crate.

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// Seed for withdraw authority seed
const AUTHORITY_WITHDRAW: &[u8] = b"withdraw";

/// Generates the withdraw authority program address for the stake pool
pub fn find_withdraw_authority_program_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[stake_pool_address.as_ref(), AUTHORITY_WITHDRAW],
        program_id,
    )
}

/// Minimal subset of SPL Stake Pool instructions needed for this program.
///
/// IMPORTANT: These variants have explicit discriminants to match the actual SPL Stake Pool program.
/// DepositSol is variant #14 and DepositSolWithSlippage is variant #25 in the real enum.
/// See: <https://github.com/solana-program/stake-pool/blob/main/program/src/instruction.rs>
///
/// Note: This appears in the IDL because it's used in public functions, but only with 2 variants.
#[repr(u8)]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
#[borsh(use_discriminant = true)]
pub enum StakePoolInstruction {
    /// Deposit SOL into the stake pool (variant #14)
    DepositSol(u64) = 14,
    /// Deposit SOL into the stake pool with slippage protection (variant #25)
    DepositSolWithSlippage {
        lamports_in: u64,
        minimum_pool_tokens_out: u64,
    } = 25,
}

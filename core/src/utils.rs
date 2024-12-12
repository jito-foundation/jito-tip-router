use solana_program::{
    account_info::AccountInfo,
    entrypoint::ProgramResult,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use crate::constants::MAX_REALLOC_BYTES;

/// Calculate new size for reallocation, capped at target size
/// Returns the minimum of (current_size + MAX_REALLOC_BYTES) and target_size
pub fn get_new_size(current_size: usize, target_size: usize) -> Result<usize, ProgramError> {
    Ok(current_size
        .checked_add(MAX_REALLOC_BYTES as usize)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .min(target_size))
}

pub fn realloc_account<'a, 'info>(
    account: &'a AccountInfo<'info>,
    payer: &'a AccountInfo<'info>,
    system_program: &'a AccountInfo<'info>,
    rent: &Rent,
    target_size: u64,
    seeds: &[Vec<u8>],
) -> ProgramResult {
    let current_size = account.data_len();

    // If account is already over target size, don't try to shrink
    if current_size >= target_size as usize {
        return Ok(());
    }

    // Calculate new size, capped at target_size
    let new_size = current_size
        .checked_add(MAX_REALLOC_BYTES as usize)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .min(target_size as usize);

    // Calculate required lamports for new size
    let new_minimum_balance = rent.minimum_balance(new_size);
    let lamports_diff = new_minimum_balance.saturating_sub(account.lamports());

    // Transfer lamports if needed
    if lamports_diff > 0 {
        invoke(
            &system_instruction::transfer(payer.key, account.key, lamports_diff),
            &[payer.clone(), account.clone(), system_program.clone()],
        )?;
    }

    // Reallocate space
    invoke_signed(
        &system_instruction::allocate(account.key, new_size as u64),
        &[account.clone(), system_program.clone()],
        &[seeds
            .iter()
            .map(|seed| seed.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;

    Ok(())
}

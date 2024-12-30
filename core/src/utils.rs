use solana_program::program_error::ProgramError;

use crate::{constants::MAX_REALLOC_BYTES, error::TipRouterError};

/// Calculate new size for reallocation, capped at target size
/// Returns the minimum of (current_size + MAX_REALLOC_BYTES) and target_size
pub fn get_new_size(current_size: usize, target_size: usize) -> Result<usize, ProgramError> {
    Ok(current_size
        .checked_add(MAX_REALLOC_BYTES as usize)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .min(target_size))
}

#[inline(always)]
#[track_caller]
pub fn assert_tip_router_error<T>(
    test_error: Result<T, TipRouterError>,
    tip_router_error: TipRouterError,
) {
    assert!(test_error.is_err());
    assert_eq!(test_error.err().unwrap(), tip_router_error);
}

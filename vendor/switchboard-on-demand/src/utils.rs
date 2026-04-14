use solana_program::instruction::Instruction;

use crate::anchor_traits::*;
use crate::solana_compat::{hash, pubkey};
use crate::{solana_program, Pubkey};

/// Check if devnet environment is enabled via feature flag OR SB_ENV environment variable
#[inline(always)]
pub fn is_devnet() -> bool {
    cfg!(feature = "devnet") || std::env::var("SB_ENV").unwrap_or_default() == "devnet"
}

/// Default devnet queue address
pub const DEFAULT_DEVNET_QUEUE: Pubkey = pubkey!("EYiAmGSdsQTuCw413V5BzaruWuCCSDgTPtBGvLkXHbe7");
/// Default mainnet queue address
pub const DEFAULT_MAINNET_QUEUE: Pubkey = pubkey!("A43DyUGA7s8eXPxqEjJY6EBu1KKbNgfxF8h17VAHn13w");

/// Returns the default queue address based on the environment (devnet or mainnet)
#[inline(always)]
pub fn default_queue() -> Pubkey {
    if is_devnet() {
        DEFAULT_DEVNET_QUEUE
    } else {
        DEFAULT_MAINNET_QUEUE
    }
}

/// SPL Associated Token Account program ID
pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");

/// SPL Token program ID
pub const SPL_TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");

pub const NATIVE_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

/// Address Lookup Table program ID
pub const ADDRESS_LOOKUP_TABLE_PROGRAM_ID: Pubkey =
    pubkey!("AddressLookupTab1e1111111111111111111111111");

/// System program ID constant (same across all versions)
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");

/// Finds the associated token account address for a given owner and mint
pub fn find_associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    let (akey, _bump) = Pubkey::find_program_address(
        &[owner.as_ref(), SPL_TOKEN_PROGRAM_ID.as_ref(), mint.as_ref()],
        &SPL_ASSOCIATED_TOKEN_ACCOUNT_PROGRAM_ID,
    );
    akey
}

/// Gets the instruction discriminator for a given instruction name
pub fn get_ixn_discriminator(ixn_name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", ixn_name);
    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}

/// Gets the account discriminator for a given account name
pub fn get_account_discriminator(account_name: &str) -> [u8; 8] {
    let id = format!("account:{}", account_name);
    hash::hash(id.as_bytes()).to_bytes()[..8]
        .try_into()
        .unwrap()
}

/// Reads a u64 value from a pointer at a given offset (unsafe)
///
/// # Safety
/// The caller must ensure that:
/// - `ptr` is a valid pointer
/// - `ptr.add(offset)` is within bounds and valid
/// - The memory at `ptr.add(offset)` contains a valid u64 value
#[inline(always)]
pub unsafe fn read_u64_at(ptr: *const u64, offset: usize) -> u64 {
    core::ptr::read_unaligned(ptr.add(offset))
}

/// Reads a u64 value from a pointer (unsafe)
///
/// # Safety
/// The caller must ensure that:
/// - `ptr` is valid and properly aligned for u64 access
/// - `ptr.add(offset)` is within bounds and valid
/// - The memory at `ptr.add(offset)` contains a valid u64 value
#[inline(always)]
pub unsafe fn read(ptr: *const u64, offset: usize) -> u64 {
    *ptr.add(offset)
}

/// Efficiently compares two Pubkeys for equality
#[inline(always)]
pub fn check_pubkey_eq<L: AsRef<[u8]>, R: AsRef<[u8]>>(lhs: L, rhs: R) -> bool {
    let lhs_bytes = lhs.as_ref();
    let rhs_bytes = rhs.as_ref();

    unsafe {
        let lhs_ptr = lhs_bytes.as_ptr() as *const u64;
        let rhs_ptr = rhs_bytes.as_ptr() as *const u64;
        core::ptr::read_unaligned(lhs_ptr) == core::ptr::read_unaligned(rhs_ptr)
            && core::ptr::read_unaligned(lhs_ptr.add(1))
                == core::ptr::read_unaligned(rhs_ptr.add(1))
            && core::ptr::read_unaligned(lhs_ptr.add(2))
                == core::ptr::read_unaligned(rhs_ptr.add(2))
            && core::ptr::read_unaligned(lhs_ptr.add(3))
                == core::ptr::read_unaligned(rhs_ptr.add(3))
    }
}

/// Efficiently compares two 32-byte arrays via u64 pointers (unsafe)
///
/// # Safety
/// The caller must ensure that:
/// - Both `lhs_ptr` and `rhs_ptr` are valid pointers
/// - Both pointers point to memory regions of at least 32 bytes (4 u64 values)
/// - The memory regions are accessible for the duration of the function call
#[inline(always)]
pub unsafe fn check_p64_eq(lhs_ptr: *const u64, rhs_ptr: *const u64) -> bool {
    core::ptr::read_unaligned(lhs_ptr) == core::ptr::read_unaligned(rhs_ptr)
        && core::ptr::read_unaligned(lhs_ptr.add(1)) == core::ptr::read_unaligned(rhs_ptr.add(1))
        && core::ptr::read_unaligned(lhs_ptr.add(2)) == core::ptr::read_unaligned(rhs_ptr.add(2))
        && core::ptr::read_unaligned(lhs_ptr.add(3)) == core::ptr::read_unaligned(rhs_ptr.add(3))
}

/// Builds a Solana instruction from account metas and instruction data
pub fn build_ix<A: ToAccountMetas, I: InstructionData + Discriminator + std::fmt::Debug>(
    program_id: &Pubkey,
    accounts: &A,
    params: &I,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: accounts.to_account_metas(None),
        data: params.data(),
    }
}

pub fn derive_lookup_table_address(
    authority_address: &Pubkey,
    recent_block_slot: u64,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[authority_address.as_ref(), &recent_block_slot.to_le_bytes()],
        &crate::ADDRESS_LOOKUP_TABLE_PROGRAM_ID,
    )
}

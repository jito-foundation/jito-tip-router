use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

/// Uninitiatilized, no-data account used to hold SOL for routing rewards
/// Must be empty and uninitialized to be used as a payer or `transfer` instructions fail
pub struct SolRouter {}

impl SolRouter {
    pub fn seeds() -> Vec<Vec<u8>> {
        vec![b"sol_router".to_vec()]
    }

    pub fn find_program_address(program_id: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds();
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if account.owner.ne(&solana_program::system_program::ID) {
            msg!("ClaimStatusPayer account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }

        if expect_writable && !account.is_writable {
            msg!("ClaimStatusPayer account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.key.ne(&Self::find_program_address(program_id).0) {
            msg!("SolRouter account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

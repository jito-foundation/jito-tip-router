use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

/// Uninitiatilized, no-data account used to hold SOL for ClaimStatus rent
/// Must be empty and uninitialized to be used as a payer or `transfer` instructions fail
pub struct ClaimStatusPayer {}

impl ClaimStatusPayer {
    pub fn seeds(tip_distribution_program: &Pubkey) -> Vec<Vec<u8>> {
        vec![
            b"claim_status_payer".to_vec(),
            tip_distribution_program.to_bytes().to_vec(),
        ]
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        tip_distribution_program: &Pubkey,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let mut seeds = Self::seeds(tip_distribution_program);
        seeds.push(tip_distribution_program.to_bytes().to_vec());
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    pub fn load(
        program_id: &Pubkey,
        account: &AccountInfo,
        tip_distribution_program: &Pubkey,
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

        if account
            .key
            .ne(&Self::find_program_address(program_id, tip_distribution_program).0)
        {
            msg!("ClaimStatusPayer account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

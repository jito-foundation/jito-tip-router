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

#[cfg(test)]
mod tests {
    use solana_program::system_program;

    use super::*;

    #[test]
    fn test_seeds() {
        let tip_distribution_program = Pubkey::new_unique();
        let seeds = ClaimStatusPayer::seeds(&tip_distribution_program);

        // Verify we get exactly 2 seeds
        assert_eq!(seeds.len(), 2);

        // Verify first seed is the string literal
        assert_eq!(seeds[0], b"claim_status_payer".to_vec());

        // Verify second seed is the pubkey bytes
        assert_eq!(seeds[1], tip_distribution_program.to_bytes().to_vec());
    }

    #[test]
    fn test_find_program_address() {
        let program_id = Pubkey::new_unique();
        let tip_distribution_program = Pubkey::new_unique();

        let (pda, bump, seeds) =
            ClaimStatusPayer::find_program_address(&program_id, &tip_distribution_program);

        // Verify we get 3 seeds (original 2 plus the tip_distribution_program bytes)
        assert_eq!(seeds.len(), 3);
        assert_eq!(seeds[0], b"claim_status_payer".to_vec());
        assert_eq!(seeds[1], tip_distribution_program.to_bytes().to_vec());
        assert_eq!(seeds[2], tip_distribution_program.to_bytes().to_vec());

        // Verify we can recreate the same PDA
        let seeds_slice: Vec<&[u8]> = seeds.iter().map(|s| s.as_slice()).collect();
        let (derived_address, derived_bump) =
            Pubkey::find_program_address(&seeds_slice, &program_id);

        assert_eq!(pda, derived_address);
        assert_eq!(bump, derived_bump);
    }

    #[test]
    fn test_load() {
        let program_id = Pubkey::new_unique();
        let tip_distribution_program = Pubkey::new_unique();
        let mut lamports = 0;
        let mut data = vec![];

        let (address, _, _) =
            ClaimStatusPayer::find_program_address(&program_id, &tip_distribution_program);

        // Test 1: Valid case
        let account = AccountInfo::new(
            &address,
            false,
            false,
            &mut lamports,
            &mut data,
            &system_program::ID,
            false,
            0,
        );

        let result =
            ClaimStatusPayer::load(&program_id, &account, &tip_distribution_program, false);
        assert!(result.is_ok());

        // Test 2: Invalid owner
        let wrong_owner = Pubkey::new_unique();
        let account = AccountInfo::new(
            &address,
            false,
            false,
            &mut lamports,
            &mut data,
            &wrong_owner,
            false,
            0,
        );

        let result =
            ClaimStatusPayer::load(&program_id, &account, &tip_distribution_program, false);
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountOwner);

        // Test 3: Not writable when expected
        let account = AccountInfo::new(
            &address,
            false,
            false, // not writable
            &mut lamports,
            &mut data,
            &system_program::ID,
            false,
            0,
        );

        let result = ClaimStatusPayer::load(
            &program_id,
            &account,
            &tip_distribution_program,
            true, // expect writable
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);

        // Test 4: Wrong PDA address
        let wrong_address = Pubkey::new_unique();
        let account = AccountInfo::new(
            &wrong_address,
            false,
            false,
            &mut lamports,
            &mut data,
            &system_program::ID,
            false,
            0,
        );

        let result =
            ClaimStatusPayer::load(&program_id, &account, &tip_distribution_program, false);
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);
    }
}

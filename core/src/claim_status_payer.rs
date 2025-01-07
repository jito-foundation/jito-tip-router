use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey, system_program,
};

use crate::loaders::check_load;

/// Uninitialized, no-data account used to hold SOL for ClaimStatus rent
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
        let system_program_id = system_program::id();
        let expected_pda = Self::find_program_address(program_id, tip_distribution_program).0;
        check_load(
            &system_program_id,
            account,
            &expected_pda,
            None,
            expect_writable,
        )
    }
}

#[cfg(test)]
mod tests {

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
}

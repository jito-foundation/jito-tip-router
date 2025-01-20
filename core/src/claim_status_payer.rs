use jito_tip_distribution_sdk::jito_tip_distribution;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program::invoke_signed,
    program_error::ProgramError, pubkey::Pubkey, rent::Rent, system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::{constants::MAX_REALLOC_BYTES, loaders::check_load};

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
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let system_program_id = system_program::id();
        let tip_distribution_program = jito_tip_distribution::ID;
        let expected_pda = Self::find_program_address(program_id, &tip_distribution_program).0;
        check_load(
            &system_program_id,
            account,
            &expected_pda,
            None,
            expect_writable,
        )
    }

    #[inline(always)]
    pub fn pay_and_create_account<'a, 'info>(
        program_id: &Pubkey,
        claim_status_payer: &'a AccountInfo<'info>,
        new_account: &'a AccountInfo<'info>,
        system_program: &'a AccountInfo<'info>,
        program_owner: &Pubkey,
        space: usize,
        new_account_seeds: &[Vec<u8>],
    ) -> ProgramResult {
        let rent = &Rent::get()?;
        let minimum_balance = rent.minimum_balance(space);
        let required_lamports = minimum_balance.saturating_sub(new_account.lamports());

        // Transfer
        if required_lamports > 0 {
            Self::transfer(
                program_id,
                claim_status_payer,
                new_account,
                required_lamports,
            )?;
        }

        // Allocate space.
        let space: u64 = (space as u64).min(MAX_REALLOC_BYTES);
        invoke_signed(
            &system_instruction::allocate(new_account.key, space),
            &[new_account.clone(), system_program.clone()],
            &[new_account_seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;

        // Assign to the specified program
        invoke_signed(
            &system_instruction::assign(new_account.key, program_owner),
            &[new_account.clone(), system_program.clone()],
            &[new_account_seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )
    }

    pub fn pay_and_realloc<'a, 'info>(
        program_id: &Pubkey,
        claim_status_payer: &'a AccountInfo<'info>,
        account: &'a AccountInfo<'info>,
        new_size: usize,
    ) -> ProgramResult {
        let rent = &Rent::get()?;
        let new_minimum_balance = rent.minimum_balance(new_size);

        let required_lamports = new_minimum_balance.saturating_sub(account.lamports());
        if required_lamports > 0 {
            Self::transfer(program_id, claim_status_payer, account, required_lamports)?;
        }

        account.realloc(new_size, false)?;
        Ok(())
    }

    /// Closes the program account
    pub fn close_account<'a, 'info>(
        program_id: &Pubkey,
        claim_status_payer: &'a AccountInfo<'info>,
        account_to_close: &'a AccountInfo<'info>,
    ) -> ProgramResult {
        // Check if the account is owned by the program
        if account_to_close.owner != program_id {
            return Err(ProgramError::IllegalOwner);
        }

        **claim_status_payer.lamports.borrow_mut() = claim_status_payer
            .lamports()
            .checked_add(account_to_close.lamports())
            .ok_or(ProgramError::ArithmeticOverflow)?;
        **account_to_close.lamports.borrow_mut() = 0;

        account_to_close.assign(&solana_program::system_program::id());
        let mut account_data = account_to_close.data.borrow_mut();
        let data_len = account_data.len();
        solana_program::program_memory::sol_memset(*account_data, 0, data_len);

        Ok(())
    }

    pub fn transfer<'a, 'info>(
        program_id: &Pubkey,
        claim_status_payer: &'a AccountInfo<'info>,
        to: &'a AccountInfo<'info>,
        lamports: u64,
    ) -> ProgramResult {
        let (claim_status_payer_address, claim_status_payer_bump, mut claim_status_payer_seeds) =
            Self::find_program_address(program_id, &jito_tip_distribution::ID);
        claim_status_payer_seeds.push(vec![claim_status_payer_bump]);

        if claim_status_payer_address.ne(claim_status_payer.key) {
            msg!("Incorrect claim status payer PDA");
            return Err(ProgramError::InvalidAccountData);
        }

        invoke_signed(
            &system_instruction::transfer(&claim_status_payer_address, to.key, lamports),
            &[claim_status_payer.clone(), to.clone()],
            &[claim_status_payer_seeds
                .iter()
                .map(|seed| seed.as_slice())
                .collect::<Vec<&[u8]>>()
                .as_slice()],
        )?;
        Ok(())
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

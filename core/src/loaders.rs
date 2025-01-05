use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::config::Config;
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::error::TipRouterError;

pub fn load_ncn_epoch(
    restaking_config: &AccountInfo,
    current_slot: u64,
    first_slot_of_ncn_epoch: Option<u64>,
) -> Result<(u64, u64), ProgramError> {
    let ncn_epoch_length = {
        let config_data = restaking_config.data.borrow();
        let config = Config::try_from_slice_unchecked(&config_data)?;
        config.epoch_length()
    };

    let current_ncn_epoch = current_slot
        .checked_div(ncn_epoch_length)
        .ok_or(TipRouterError::DenominatorIsZero)?;

    let ncn_epoch_slot = first_slot_of_ncn_epoch.unwrap_or(current_slot);
    let ncn_epoch = ncn_epoch_slot
        .checked_div(ncn_epoch_length)
        .ok_or(TipRouterError::DenominatorIsZero)?;

    if ncn_epoch > current_ncn_epoch {
        msg!("Epoch snapshots can only be initialized for current or past epochs");
        return Err(TipRouterError::CannotCreateFutureWeightTables.into());
    }

    Ok((ncn_epoch, ncn_epoch_length))
}

pub fn check_load(
    program_id: &Pubkey,
    account: &AccountInfo,
    expected_pda: &Pubkey,
    expected_discriminator: Option<u8>,
    expect_writable: bool,
) -> Result<(), ProgramError> {
    if account.owner.ne(program_id) {
        msg!("Account has an invalid owner");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if account.key.ne(expected_pda) {
        msg!("Account is not at the correct PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    if let Some(discriminator) = expected_discriminator {
        if account.data_is_empty() {
            msg!("Account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }

        if account.data.borrow()[0].ne(&discriminator) {
            msg!("Account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
    }

    if expect_writable && !account.is_writable {
        msg!("Account is not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_load() {
        let program_id = Pubkey::new_unique();
        let mut lamports = 0;
        const DISCRIMINATOR: u8 = 0x55;
        let expected_pda = Pubkey::new_unique();
        let mut data = [0u8; 1];
        data[0] = DISCRIMINATOR;

        // Load OK with discriminator
        let account = AccountInfo::new(
            &expected_pda,
            false,
            true,
            &mut lamports,
            &mut data,
            &program_id,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            false,
        );
        assert!(result.is_ok());

        // Load OK without discriminator
        let result = check_load(&program_id, &account, &expected_pda, None, false);
        assert!(result.is_ok());

        // Invalid Owner
        let bad_owner = Pubkey::new_unique();
        let account = AccountInfo::new(
            &expected_pda,
            false,
            false,
            &mut lamports,
            &mut data,
            &bad_owner,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            false,
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountOwner);

        // Empty Data (only matters when discriminator is required)
        let mut bad_data = [0u8; 0];
        let account = AccountInfo::new(
            &expected_pda,
            false,
            false,
            &mut lamports,
            &mut bad_data,
            &program_id,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            false,
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);

        // Empty Data should be OK when no discriminator is required
        let result = check_load(&program_id, &account, &expected_pda, None, false);
        assert!(result.is_ok());

        // Not Writable when required
        let account = AccountInfo::new(
            &expected_pda,
            false,
            false,
            &mut lamports,
            &mut data,
            &program_id,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            true,
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);

        // Invalid Discriminator
        let mut bad_data = [0u8; 1];
        bad_data[0] = DISCRIMINATOR + 1;
        let account = AccountInfo::new(
            &expected_pda,
            false,
            false,
            &mut lamports,
            &mut bad_data,
            &program_id,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            false,
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);

        // Invalid PDA
        let wrong_pda = Pubkey::new_unique();
        let account = AccountInfo::new(
            &wrong_pda,
            false,
            false,
            &mut lamports,
            &mut data,
            &program_id,
            false,
            0,
        );

        let result = check_load(
            &program_id,
            &account,
            &expected_pda,
            Some(DISCRIMINATOR),
            false,
        );
        assert_eq!(result.err().unwrap(), ProgramError::InvalidAccountData);
    }
}

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::{discriminators::Discriminators, fees::FeeConfig};

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct NcnConfig {
    /// The Restaking program's NCN admin is the signer to create and update this account
    pub ncn: Pubkey,

    pub tie_breaker_admin: Pubkey,

    pub fee_admin: Pubkey,

    /// Number of slots after consensus reached where voting is still valid
    pub valid_slots_after_consensus: PodU64,

    /// Number of epochs before voting is considered stalled
    pub epochs_before_stall: PodU64,

    pub fee_config: FeeConfig,

    /// Bump seed for the PDA
    pub bump: u8,
    // /// Reserved space
    reserved: [u8; 127],
}

impl Discriminator for NcnConfig {
    const DISCRIMINATOR: u8 = Discriminators::NCNConfig as u8;
}

impl NcnConfig {
    pub fn new(
        ncn: Pubkey,
        tie_breaker_admin: Pubkey,
        fee_admin: Pubkey,
        fee_config: &FeeConfig,
    ) -> Self {
        Self {
            ncn,
            tie_breaker_admin,
            fee_admin,
            valid_slots_after_consensus: PodU64::from(0), // TODO set this
            epochs_before_stall: PodU64::from(0),         // TODO set this
            fee_config: *fee_config,
            bump: 0,
            reserved: [0; 127],
        }
    }

    pub fn seeds(ncn: &Pubkey) -> Vec<Vec<u8>> {
        vec![b"config".to_vec(), ncn.to_bytes().to_vec()]
    }

    pub fn find_program_address(program_id: &Pubkey, ncn: &Pubkey) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let seeds = Self::seeds(ncn);
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    /// Loads the NCN [`Config`] account
    ///
    /// # Arguments
    /// * `program_id` - The program ID
    /// * `ncn` - The NCN pubkey
    /// * `account` - The account to load
    /// * `expect_writable` - Whether the account should be writable
    ///
    /// # Returns
    /// * `Result<(), ProgramError>` - The result of the operation
    pub fn load(
        program_id: &Pubkey,
        ncn: &Pubkey,
        ncn_config_account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        if ncn_config_account.owner.ne(program_id) {
            msg!("NCN Config account has an invalid owner");
            return Err(ProgramError::InvalidAccountOwner);
        }
        if ncn_config_account.data_is_empty() {
            msg!("NCN Config account data is empty");
            return Err(ProgramError::InvalidAccountData);
        }
        if expect_writable && !ncn_config_account.is_writable {
            msg!("NCN Config account is not writable");
            return Err(ProgramError::InvalidAccountData);
        }
        if ncn_config_account.data.borrow()[0].ne(&Self::DISCRIMINATOR) {
            msg!("NCN Config account discriminator is invalid");
            return Err(ProgramError::InvalidAccountData);
        }
        if ncn_config_account
            .key
            .ne(&Self::find_program_address(program_id, ncn).0)
        {
            msg!("NCN Config account is not at the correct PDA");
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }

    pub fn valid_slots_after_consensus(&self) -> u64 {
        self.valid_slots_after_consensus.into()
    }

    pub fn epochs_before_stall(&self) -> u64 {
        self.epochs_before_stall.into()
    }
}

use std::mem::size_of;

use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey};

use crate::{discriminators::Discriminators, fees::FeeConfig, loaders::check_load};

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum ConfigAdminRole {
    FeeAdmin,
    TieBreakerAdmin,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct Config {
    /// The Restaking program's NCN admin is the signer to create and update this account
    pub ncn: Pubkey,
    /// The admin to update the tie breaker - who can decide the meta merkle root when consensus is reached
    pub tie_breaker_admin: Pubkey,
    /// The admin to update the fee config
    pub fee_admin: Pubkey,
    /// Number of slots after consensus reached where voting is still valid
    pub valid_slots_after_consensus: PodU64,
    /// Number of epochs before voting is considered stalled
    pub epochs_before_stall: PodU64,
    /// The fee config
    pub fee_config: FeeConfig,
    /// Bump seed for the PDA
    pub bump: u8,
    /// Reserved space
    reserved: [u8; 127],
}

impl Discriminator for Config {
    const DISCRIMINATOR: u8 = Discriminators::Config as u8;
}

impl Config {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(
        ncn: &Pubkey,
        tie_breaker_admin: &Pubkey,
        fee_admin: &Pubkey,
        fee_config: &FeeConfig,
        valid_slots_after_consensus: u64,
        epochs_before_stall: u64,
        bump: u8,
    ) -> Self {
        Self {
            ncn: *ncn,
            tie_breaker_admin: *tie_breaker_admin,
            fee_admin: *fee_admin,
            valid_slots_after_consensus: PodU64::from(valid_slots_after_consensus),
            epochs_before_stall: PodU64::from(epochs_before_stall),
            fee_config: *fee_config,
            bump,
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
        account: &AccountInfo,
        expect_writable: bool,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn).0;
        check_load(
            program_id,
            account,
            &expected_pda,
            Some(Self::DISCRIMINATOR),
            expect_writable,
        )
    }

    pub fn valid_slots_after_consensus(&self) -> u64 {
        self.valid_slots_after_consensus.into()
    }

    pub fn epochs_before_stall(&self) -> u64 {
        self.epochs_before_stall.into()
    }
}

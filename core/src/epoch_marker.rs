use std::mem::size_of;

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::{types::PodU64, AccountDeserialize, Discriminator};
use shank::{ShankAccount, ShankType};
use solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey};

use crate::discriminators::Discriminators;

/// 56-byte account to mark that an epoch's accounts have all been closed
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod, AccountDeserialize, ShankAccount)]
#[repr(C)]
pub struct EpochMarker {
    ncn: Pubkey,
    epoch: PodU64,
    slot_closed: PodU64,
}

impl Discriminator for EpochMarker {
    const DISCRIMINATOR: u8 = Discriminators::EpochMarker as u8;
}

impl EpochMarker {
    pub const SIZE: usize = 8 + size_of::<Self>();

    pub fn new(ncn: &Pubkey, epoch: u64, slot_closed: u64) -> Self {
        Self {
            ncn: *ncn,
            epoch: PodU64::from(epoch),
            slot_closed: PodU64::from(slot_closed),
        }
    }

    pub const fn ncn(&self) -> &Pubkey {
        &self.ncn
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.into()
    }

    pub fn slot_closed(&self) -> u64 {
        self.slot_closed.into()
    }

    pub fn seeds(ncn: &Pubkey, epoch: u64) -> Vec<Vec<u8>> {
        vec![
            b"epoch_marker".to_vec(),
            ncn.to_bytes().to_vec(),
            epoch.to_le_bytes().to_vec(),
        ]
    }

    pub fn find_program_address(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
    ) -> (Pubkey, u8, Vec<Vec<u8>>) {
        let mut seeds = Self::seeds(ncn, epoch);
        seeds.push(ncn.to_bytes().to_vec());
        let (address, bump) = Pubkey::find_program_address(
            &seeds.iter().map(|s| s.as_slice()).collect::<Vec<_>>(),
            program_id,
        );
        (address, bump, seeds)
    }

    pub fn check_dne(
        program_id: &Pubkey,
        ncn: &Pubkey,
        epoch: u64,
        account: &AccountInfo,
    ) -> Result<(), ProgramError> {
        let expected_pda = Self::find_program_address(program_id, ncn, epoch).0;

        if expected_pda.ne(account.key) {
            msg!(
                "Epoch marker PDA does not match {} != {}",
                account.key,
                expected_pda
            );
            return Err(ProgramError::InvalidSeeds);
        }

        let data_length = account.data_len();
        if data_length > 0 {
            msg!("Market exists.");
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        Ok(())
    }
}

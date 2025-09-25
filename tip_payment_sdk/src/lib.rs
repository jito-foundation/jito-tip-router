use anyhow::Result;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

pub const CONFIG_ACCOUNT_SEED: &[u8] = b"CONFIG_ACCOUNT";
pub const TIP_ACCOUNT_SEED_0: &[u8] = b"TIP_ACCOUNT_0";
pub const TIP_ACCOUNT_SEED_1: &[u8] = b"TIP_ACCOUNT_1";
pub const TIP_ACCOUNT_SEED_2: &[u8] = b"TIP_ACCOUNT_2";
pub const TIP_ACCOUNT_SEED_3: &[u8] = b"TIP_ACCOUNT_3";
pub const TIP_ACCOUNT_SEED_4: &[u8] = b"TIP_ACCOUNT_4";
pub const TIP_ACCOUNT_SEED_5: &[u8] = b"TIP_ACCOUNT_5";
pub const TIP_ACCOUNT_SEED_6: &[u8] = b"TIP_ACCOUNT_6";
pub const TIP_ACCOUNT_SEED_7: &[u8] = b"TIP_ACCOUNT_7";

pub const HEADER_SIZE: usize = 8;
pub const CONFIG_SIZE: usize = HEADER_SIZE + std::mem::size_of::<Config>();
pub const TIP_PAYMENT_ACCOUNT_SIZE: usize = HEADER_SIZE + std::mem::size_of::<TipPaymentAccount>();

#[derive(BorshSerialize, BorshDeserialize)]
pub struct InitBumps {
    pub config: u8,
    pub tip_payment_account_0: u8,
    pub tip_payment_account_1: u8,
    pub tip_payment_account_2: u8,
    pub tip_payment_account_3: u8,
    pub tip_payment_account_4: u8,
    pub tip_payment_account_5: u8,
    pub tip_payment_account_6: u8,
    pub tip_payment_account_7: u8,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Config {
    /// The account claiming tips from the mev_payment accounts.
    pub tip_receiver: Pubkey,

    /// Block builder that receives a % of fees
    pub block_builder: Pubkey,
    pub block_builder_commission_pct: u64,

    /// Bumps used to derive PDAs
    pub bumps: InitBumps,
}

impl Config {
    pub const DISCRIMINATOR: [u8; 8] = [0x9b, 0x0c, 0xaa, 0xe0, 0x1e, 0xfa, 0xcc, 0x82];

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        anyhow::ensure!(data.len() >= 8, "Account data too short");
        anyhow::ensure!(data.len() >= CONFIG_SIZE, "Invalid account size");
        let (discriminator, mut remainder) = data.split_at(8);
        anyhow::ensure!(
            discriminator == Self::DISCRIMINATOR,
            "Invalid discriminator"
        );
        Ok(<Self as BorshDeserialize>::deserialize(&mut remainder)?)
    }
}

#[derive(BorshSerialize, BorshDeserialize, Default)]
pub struct TipPaymentAccount {}

pub fn id() -> Pubkey {
    Pubkey::from_str("T1pyyaTNZsKv2WcRAB8oVnk93mLJw2XzjtVYqCsaHqt")
        .expect("Failed to parse program id")
}

#![allow(clippy::redundant_pub_crate)]
use anchor_lang::declare_program;

declare_program!(jito_tip_payment);

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
pub const CONFIG_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<jito_tip_payment::accounts::Config>();
pub const TIP_PAYMENT_ACCOUNT_SIZE: usize =
    HEADER_SIZE + std::mem::size_of::<jito_tip_payment::accounts::TipPaymentAccount>();

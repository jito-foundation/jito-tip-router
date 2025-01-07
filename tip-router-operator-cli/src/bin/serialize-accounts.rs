use std::{fs::File, io::Write, str::FromStr};

use anchor_lang::prelude::*;
use clap::Parser;
use jito_tip_distribution_sdk::{
    derive_tip_distribution_account_address, TipDistributionAccount, TIP_DISTRIBUTION_SIZE,
};
use log::info;
use serde_json::json;
use solana_program::pubkey::Pubkey;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'v', long)]
    validator_vote_account: String,

    #[arg(short = 'm', long)]
    merkle_root_upload_authority: String,

    #[arg(short = 'e', long)]
    epoch_created_at: u64,

    #[arg(short = 'c', long)]
    validator_commission_bps: u16,

    #[arg(short = 'x', long)]
    expires_at: u64,

    #[arg(short = 'b', long)]
    bump: u8,

    #[arg(long)]
    tda_accounts_dir: String,
}

fn main() {
    let args = Args::parse();

    let validator_vote_account =
        Pubkey::from_str(&args.validator_vote_account).expect("Invalid pubkey");
    let merkle_root_upload_authority =
        Pubkey::from_str(&args.merkle_root_upload_authority).expect("Invalid pubkey");

    let account = TipDistributionAccount {
        validator_vote_account,
        merkle_root_upload_authority,
        merkle_root: None,
        epoch_created_at: args.epoch_created_at,
        validator_commission_bps: args.validator_commission_bps,
        expires_at: args.expires_at,
        bump: args.bump,
    };

    let tip_distribution_program_id =
        Pubkey::from_str("4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7").unwrap();
    let current_epoch = args.epoch_created_at; // Use the epoch from args or another source
    let tip_distribution_pubkey = derive_tip_distribution_account_address(
        &tip_distribution_program_id,
        &validator_vote_account,
        current_epoch,
    )
    .0;

    // Serialize using AnchorSerialize
    let mut binary_data = [0u8; TIP_DISTRIBUTION_SIZE];
    let dst: &mut [u8] = &mut binary_data;
    let mut cursor = std::io::Cursor::new(dst);
    account
        .try_serialize(&mut cursor)
        .expect("Failed to serialize account");

    // Encode the binary data as base64
    let base64_data = base64::encode(binary_data);

    // Create the JSON structure
    let json_data = json!({
        "pubkey": tip_distribution_pubkey.to_string(),
        "account": {
            "lamports": 22451877,
            "data": [base64_data, "base64"],
            "owner": args.merkle_root_upload_authority,
            "executable": false,
            "rentEpoch": 0,  // Replace with actual rent epoch if available
            "space": binary_data.len()
        }
    });

    // Write the JSON data to a file
    // Use the validator_vote_account as part of the filename
    let filename = format!("{}/{}.json", args.tda_accounts_dir, tip_distribution_pubkey);

    // Write the JSON data to a unique file
    let mut file = File::create(&filename).unwrap();
    file.write_all(json_data.to_string().as_bytes()).unwrap();

    info!(
        "Serialized TipDistributionAccount to JSON format in file: {}",
        filename
    );
}

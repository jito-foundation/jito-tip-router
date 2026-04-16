//! Demonstrates the epoch off-by-one bug in `get_tip_distribution_accounts_to_upload`.
//!
//! On-chain TDAs for epoch N's tip distributions are created during epoch N+1, so their
//! `epoch_created_at` field equals `tip_router_target_epoch` (= merkle_root_epoch + 1),
//! NOT `merkle_root_epoch`. The original code passed `merkle_root_epoch`, so the
//! `getProgramAccounts` filter never matched and 0 accounts were returned.
//!
//! Evidence: `solana account D3dUAFeNBcwFJURXeXny9ABejQHR5GLGfjUhvbYC1cz8 --url mainnet-beta`
//!   bytes [72]    = 0x00          → merkle_root = None
//!   bytes [73-80] = bd 03 00 ...  → epoch_created_at = 957  (not 956)
//!
//! Run:
//!   cargo run -p tip-router-operator-cli --example get_tip_distribution_accounts_to_upload
//!
//! Optional env vars:
//!   RPC_URL                  (default: https://api.mainnet-beta.solana.com)
//!   MERKLE_ROOT_EPOCH        (default: 956)

use std::str::FromStr;

use jito_tip_distribution_sdk::TipDistributionAccount;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;
use tip_router_operator_cli::submit::get_tip_distribution_accounts_to_upload;

// Owner of TDA D3dUAFeNBcwFJURXeXny9ABejQHR5GLGfjUhvbYC1cz8
const TIP_DISTRIBUTION_PROGRAM_ID: &str = "4R3gSG8BpU4t19KYj8CfnbtRpnT8gtk4dvTHxVRwc2r7";

// bytes [40-71] of D3dUAFeNBcwFJURXeXny9ABejQHR5GLGfjUhvbYC1cz8 = merkle_root_upload_authority
// = tip_router_config_address (Config::find_program_address(tip_router_program_id, ncn))
const TIP_ROUTER_CONFIG_ADDRESS: &str = "8F4jGUmxF36vQ6yabnsxX6AQVXdKBhs8kGSUuRKSg8Xt";

/// Replicates the logic in `submit::get_tip_distribution_accounts_to_upload`.
///
/// Filter layout (TDA on-chain data has an 8-byte Anchor discriminator prefix):
///   offset  8 + 32       = 40  → merkle_root_upload_authority (32 bytes)
///   offset  8 + 32 + 32  = 72  → Option tag (0x00 = None)
///   offset  8 + 32 + 32 + 1 = 73 → epoch_created_at u64 LE  ← only here when tag = None
async fn fetch_tdas(
    client: &RpcClient,
    epoch: u64,
    tip_router_config: &Pubkey,
    tip_dist_program: &Pubkey,
) -> anyhow::Result<Vec<(Pubkey, TipDistributionAccount)>> {
    let filters = vec![
        // Filter 1: merkle_root_upload_authority == tip_router_config_address
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 + 32, // discriminator + validator_vote_account
            tip_router_config.to_bytes().to_vec(),
        )),
        // Filter 2: epoch_created_at == epoch
        // This byte position is only valid when merkle_root = None (Option tag 0x00),
        // so it implicitly filters for un-uploaded TDAs at the same time.
        RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            8 + 32 + 32 + 1, // discriminator + validator_vote_account + upload_authority + Option tag
            epoch.to_le_bytes().to_vec(),
        )),
    ];

    let raw_accounts = client
        .get_program_accounts_with_config(
            tip_dist_program,
            RpcProgramAccountsConfig {
                filters: Some(filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await?;

    let accounts = raw_accounts
        .into_iter()
        .filter_map(|(pubkey, account)| {
            let tda = TipDistributionAccount::deserialize(&account.data).ok()?;
            // Post-filter: double-check fields match (guards against hash collisions in filter 2)
            if tda.epoch_created_at == epoch
                && tda.merkle_root_upload_authority == *tip_router_config
            {
                Some((pubkey, tda))
            } else {
                eprintln!("  [warn] post-filter mismatch for {pubkey}");
                None
            }
        })
        .collect();

    Ok(accounts)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let rpc_url = std::env::var("RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());

    let merkle_root_epoch: u64 = std::env::var("MERKLE_ROOT_EPOCH")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(956);

    let tip_router_target_epoch = merkle_root_epoch + 1;

    let client = RpcClient::new(rpc_url.clone());
    let tip_router_config = Pubkey::from_str(TIP_ROUTER_CONFIG_ADDRESS).unwrap();
    let tip_dist_program: Pubkey = TIP_DISTRIBUTION_PROGRAM_ID.parse()?;

    println!("RPC:                   {rpc_url}");
    println!("tip_dist_program:      {tip_dist_program}");
    println!("tip_router_config:     {tip_router_config}");
    println!("merkle_root_epoch:     {merkle_root_epoch}");
    println!("tip_router_target_epoch: {tip_router_target_epoch}");
    println!();

    // Quick debug test — remove filters
    let tip_distribution_accounts = client.get_program_accounts(&tip_dist_program).await?;
    println!("Total accounts: {}", tip_distribution_accounts.len());

    // --- Wrong: current code passes merkle_root_epoch ---
    println!("=== epoch={merkle_root_epoch} (merkle_root_epoch — the bug) ===");
    let wrong = get_tip_distribution_accounts_to_upload(
        &client,
        tip_router_target_epoch,
        &tip_router_config,
        &tip_dist_program,
    )
    .await?;
    println!("Found: {}", wrong.len());
    for (pubkey, tda) in &wrong {
        println!(
            "  {pubkey}  epoch_created_at={}  merkle_root={}",
            tda.epoch_created_at,
            if tda.merkle_root.is_some() {
                "Some"
            } else {
                "None"
            },
        );
    }
    println!();

    // --- Correct: should pass tip_router_target_epoch ---
    // println!("=== epoch={tip_router_target_epoch} (tip_router_target_epoch — the fix) ===");
    // let correct = get_tip_distribution_accounts_to_upload(
    //     &client,
    //     tip_router_target_epoch,
    //     &tip_router_config,
    //     &tip_dist_program,
    // )
    // .await?;
    // println!("Found: {}", correct.len());
    // for (pubkey, tda) in &correct {
    //     println!(
    //         "  {pubkey}  epoch_created_at={}  merkle_root={}",
    //         tda.epoch_created_at,
    //         if tda.merkle_root.is_some() {
    //             "Some"
    //         } else {
    //             "None"
    //         },
    //     );
    // }

    Ok(())
}

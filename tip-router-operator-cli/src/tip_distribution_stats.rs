use anyhow::Result;
use anchor_lang::AccountDeserialize;
use jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount;
use jito_tip_distribution_sdk::TipDistributionAccount;
use log::info;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::pubkey::Pubkey;

pub struct TipDistributionStats {
    pub account_pubkey: Pubkey,
    pub validator_vote_account: Pubkey,
    pub total_lamports: u64,
    pub is_priority_fee: bool,
    pub validator_commission_bps: u16,
}

pub async fn get_tip_distribution_stats(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<()> {
    info!("Fetching tip distribution accounts for epoch {}...", epoch);

    // Create a new RPC client with 1800 second timeout
    let rpc_client_with_timeout = RpcClient::new_with_timeout(
        rpc_client.url().to_string(),
        std::time::Duration::from_secs(1800),
    );

    // Get tip distribution accounts
    let tip_distribution_accounts = get_tip_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        tip_distribution_program_id,
        epoch,
    )
    .await?;

    // Get priority fee distribution accounts
    let priority_fee_distribution_accounts = get_priority_fee_distribution_accounts_for_epoch(
        &rpc_client_with_timeout,
        priority_fee_distribution_program_id,
        epoch,
    )
    .await?;

    info!(
        "Found {} tip distribution accounts and {} priority fee distribution accounts",
        tip_distribution_accounts.len(),
        priority_fee_distribution_accounts.len()
    );

    // Process tip distribution accounts
    let mut all_stats = Vec::new();
    
    for (pubkey, account) in tip_distribution_accounts {
        let stats = process_tip_distribution_account(
            &rpc_client_with_timeout,
            &pubkey,
            &account,
            tip_distribution_program_id,
            false,
        )
        .await?;
        // Only include accounts with non-zero total lamports
        if stats.total_lamports > 0 {
            all_stats.push(stats);
        }
    }

    // Process priority fee distribution accounts
    for (pubkey, account) in priority_fee_distribution_accounts {
        let stats = process_priority_fee_distribution_account(
            &rpc_client_with_timeout,
            &pubkey,
            &account,
            priority_fee_distribution_program_id,
            true,
        )
        .await?;
        // Only include accounts with non-zero total lamports
        if stats.total_lamports > 0 {
            all_stats.push(stats);
        }
    }

    // Print summary
    print_tip_distribution_summary(&all_stats, epoch);

    Ok(())
}

async fn get_tip_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    tip_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, TipDistributionAccount)>> {
    // Get all accounts without filters first, then filter in memory
    // This is more reliable than trying to get the exact filter right
    let accounts = rpc_client
        .get_program_accounts_with_config(
            tip_distribution_program_id,
            RpcProgramAccountsConfig {
                filters: None,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await?;

    let mut result = Vec::new();
    for (pubkey, account) in accounts {
        if let Ok(tip_distribution_account) = TipDistributionAccount::try_deserialize(
            &mut account.data.as_slice(),
        ) {
            if tip_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, tip_distribution_account));
            }
        }
    }

    Ok(result)
}

async fn get_priority_fee_distribution_accounts_for_epoch(
    rpc_client: &RpcClient,
    priority_fee_distribution_program_id: &Pubkey,
    epoch: u64,
) -> Result<Vec<(Pubkey, PriorityFeeDistributionAccount)>> {
    // Get all accounts without filters first, then filter in memory
    // This is more reliable than trying to get the exact filter right
    let accounts = rpc_client
        .get_program_accounts_with_config(
            priority_fee_distribution_program_id,
            RpcProgramAccountsConfig {
                filters: None,
                account_config: RpcAccountInfoConfig {
                    encoding: Some(solana_account_decoder::UiAccountEncoding::Base64),
                    ..RpcAccountInfoConfig::default()
                },
                ..RpcProgramAccountsConfig::default()
            },
        )
        .await?;

    let mut result = Vec::new();
    for (pubkey, account) in accounts {
        if let Ok(priority_fee_distribution_account) = PriorityFeeDistributionAccount::try_deserialize(
            &mut account.data.as_slice(),
        ) {
            if priority_fee_distribution_account.epoch_created_at == epoch {
                result.push((pubkey, priority_fee_distribution_account));
            }
        }
    }

    Ok(result)
}

async fn process_tip_distribution_account(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    tip_distribution_account: &TipDistributionAccount,
    program_id: &Pubkey,
    is_priority_fee: bool,
) -> Result<TipDistributionStats> {
    // Get the account data to calculate total lamports
    let account_info = rpc_client.get_account(account_pubkey).await?;
    let rent_exempt_amount = rpc_client
        .get_minimum_balance_for_rent_exemption(account_info.data.len())
        .await?;
    let total_lamports = account_info.lamports.saturating_sub(rent_exempt_amount);

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: tip_distribution_account.validator_vote_account,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: tip_distribution_account.validator_commission_bps,
    })
}

async fn process_priority_fee_distribution_account(
    rpc_client: &RpcClient,
    account_pubkey: &Pubkey,
    priority_fee_distribution_account: &PriorityFeeDistributionAccount,
    program_id: &Pubkey,
    is_priority_fee: bool,
) -> Result<TipDistributionStats> {
    // Get the account data to calculate total lamports
    let account_info = rpc_client.get_account(account_pubkey).await?;
    let rent_exempt_amount = rpc_client
        .get_minimum_balance_for_rent_exemption(account_info.data.len())
        .await?;
    let total_lamports = account_info.lamports.saturating_sub(rent_exempt_amount);

    Ok(TipDistributionStats {
        account_pubkey: *account_pubkey,
        validator_vote_account: priority_fee_distribution_account.validator_vote_account,
        total_lamports,
        is_priority_fee,
        validator_commission_bps: priority_fee_distribution_account.validator_commission_bps,
    })
}



fn print_tip_distribution_summary(stats: &[TipDistributionStats], epoch: u64) {
    println!("\n=== Epoch {} Tip Distribution Statistics ===", epoch);
    println!("{:<50} {:<15} {:<10} {:<10}", 
        "Account", "Total (SOL)", "Type", "Commission");
    println!("{:-<85}", "");

    let mut total_total = 0u64;

    for stat in stats {
        let total_sol = stat.total_lamports as f64 / 1_000_000_000.0;
        let account_type = if stat.is_priority_fee { "Priority" } else { "Tip" };
        let commission_pct = stat.validator_commission_bps as f64 / 100.0;

        println!("{:<50} {:<15.6} {:<10} {:<10.2}",
            format!("{}", stat.account_pubkey),
            total_sol,
            account_type,
            commission_pct
        );

        total_total += stat.total_lamports;
    }

    println!("{:-<85}", "");
    let total_total_sol = total_total as f64 / 1_000_000_000.0;
    
    println!("{:<50} {:<15.6} {:<10} {:<10}",
        "TOTAL",
        total_total_sol,
        "ALL",
        ""
    );
} 
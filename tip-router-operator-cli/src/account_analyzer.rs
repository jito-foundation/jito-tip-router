use anchor_lang::AnchorDeserialize;
use anyhow::Result;
use jito_tip_distribution_sdk::{
    derive_claim_status_account_address, derive_tip_distribution_account_address,
    jito_tip_distribution::accounts::ClaimStatus, TIP_DISTRIBUTION_SIZE,
};
use jito_tip_router_core::base_reward_router::BaseRewardReceiver;
use log::info;
use serde::{Deserialize, Serialize};
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{pubkey::Pubkey, rent::Rent, sysvar::Sysvar};
use std::{str::FromStr, sync::Arc};

#[derive(Debug, Deserialize, Serialize)]
pub struct AccountAnalysisEntry {
    pub vote_account: String,
    pub mev_revenue: String,
    pub claim_status_account: String,
}

#[derive(Debug)]
pub struct AccountAnalysisResult {
    pub vote_account: Pubkey,
    pub mev_revenue: u64,
    pub claim_status_account: Pubkey,
    pub tip_distribution_account: Pubkey,
    pub tip_distribution_lamports: u64,
    pub tip_distribution_rent_exempt: u64,
    pub tip_distribution_available_funds: u64,
    pub funding_deficit: u64,
    pub claim_status_exists: bool,
    pub base_reward_receiver_claim_status_exists: bool,
    pub base_reward_claim_status: Pubkey,
}

pub struct AccountAnalyzer {
    rpc_client: Arc<RpcClient>,
    tip_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn_address: Pubkey,
    epoch: u64,
}

impl AccountAnalyzer {
    pub fn new(
        rpc_client: Arc<RpcClient>,
        tip_distribution_program_id: Pubkey,
        tip_router_program_id: Pubkey,
        ncn_address: Pubkey,
        epoch: u64,
    ) -> Self {
        Self {
            rpc_client,
            tip_distribution_program_id,
            tip_router_program_id,
            ncn_address,
            epoch,
        }
    }

    pub async fn analyze_accounts(
        &self,
        entries: Vec<AccountAnalysisEntry>,
    ) -> Result<Vec<AccountAnalysisResult>> {
        let mut results = Vec::new();

        for entry in entries {
            let vote_account = Pubkey::from_str(&entry.vote_account)?;
            let mev_revenue = entry.mev_revenue.parse::<u64>()?;
            let claim_status_account = Pubkey::from_str(&entry.claim_status_account)?;

            let result = self
                .analyze_single_account(vote_account, mev_revenue, claim_status_account)
                .await?;
            results.push(result);
        }

        Ok(results)
    }

    async fn analyze_single_account(
        &self,
        vote_account: Pubkey,
        mev_revenue: u64,
        claim_status_account: Pubkey,
    ) -> Result<AccountAnalysisResult> {
        // Get tip distribution account address
        let (tip_distribution_account, _) = derive_tip_distribution_account_address(
            &self.tip_distribution_program_id,
            &vote_account,
            self.epoch,
        );

        // Get base reward receiver address
        let (base_reward_receiver_address, _, _) = BaseRewardReceiver::find_program_address(
            &self.tip_router_program_id,
            &self.ncn_address,
            self.epoch + 1,
        );

        // Fetch tip distribution account
        let tip_distribution_account_result = self
            .rpc_client
            .get_account(&tip_distribution_account)
            .await?;
        let tip_distribution_lamports = tip_distribution_account_result.lamports;

        let tip_distribution_rent_exempt = 2060160;

        let tip_distribution_available_funds =
            tip_distribution_lamports.saturating_sub(tip_distribution_rent_exempt);

        // Check if claim status account exists
        let claim_status_exists = self
            .rpc_client
            .get_account(&claim_status_account)
            .await
            .is_ok();

        // let claim_status: ClaimStatus = ClaimStatus::deserialize(&mut &claim_status_account())?;

        let funding_deficit = if !claim_status_exists {
            mev_revenue.saturating_sub(tip_distribution_available_funds)
        } else {
            0
        };

        // Check if base reward receiver claim status exists
        let (base_reward_claim_status, _) = derive_claim_status_account_address(
            &self.tip_distribution_program_id,
            &base_reward_receiver_address,
            &tip_distribution_account,
        );
        let base_reward_receiver_claim_status_exists = self
            .rpc_client
            .get_account(&base_reward_claim_status)
            .await
            .is_ok();

        Ok(AccountAnalysisResult {
            vote_account,
            mev_revenue,
            claim_status_account,
            tip_distribution_account,
            tip_distribution_lamports,
            tip_distribution_rent_exempt,
            tip_distribution_available_funds,
            funding_deficit,
            claim_status_exists,
            base_reward_receiver_claim_status_exists,
            base_reward_claim_status,
        })
    }

    pub fn print_analysis_results(&self, results: &[AccountAnalysisResult]) {
        info!("=== Account Analysis Results for Epoch {} ===", self.epoch);
        info!("NCN Address: {}", self.ncn_address);
        info!(
            "Tip Distribution Program: {}",
            self.tip_distribution_program_id
        );
        info!("Tip Router Program: {}", self.tip_router_program_id);
        info!("");

        for (i, result) in results.iter().enumerate() {
            info!("Entry {}:", i + 1);
            info!("  Vote Account: {}", result.vote_account);
            info!("  MEV Revenue: {} lamports", result.mev_revenue);
            info!(
                "  Tip Distribution Account: {}",
                result.tip_distribution_account
            );
            info!(
                "  Tip Distribution Lamports: {} (rent exempt: {})",
                result.tip_distribution_lamports, result.tip_distribution_rent_exempt
            );
            info!(
                "  Available Funds: {} lamports",
                result.tip_distribution_available_funds
            );
            info!("  Funding Deficit: {} lamports", result.funding_deficit);
            info!(
                "  Claim Status Account: {} (exists: {})",
                result.claim_status_account, result.claim_status_exists
            );
            info!(
                "  Base Reward Receiver: {}",
                result.base_reward_claim_status
            );
            info!(
                "  Base Reward Receiver Claim Status: {} (exists: {})",
                result.base_reward_claim_status, result.base_reward_receiver_claim_status_exists
            );
            info!("");
        }

        // Summary statistics
        let total_mev_revenue: u64 = results.iter().map(|r| r.mev_revenue).sum();
        let total_available_funds: u64 = results
            .iter()
            .map(|r| r.tip_distribution_available_funds)
            .sum();
        let total_funding_deficit: u64 = results.iter().map(|r| r.funding_deficit).sum();
        let accounts_with_deficit = results.iter().filter(|r| r.funding_deficit > 0).count();
        let accounts_with_claim_status = results.iter().filter(|r| r.claim_status_exists).count();
        let accounts_with_base_reward_claim_status = results
            .iter()
            .filter(|r| r.base_reward_receiver_claim_status_exists)
            .count();

        info!("=== Summary ===");
        info!("Total MEV Revenue: {} lamports", total_mev_revenue);
        info!(
            "Total Available Funds: {:9} SOL",
            total_available_funds as f64 / 1_000_000_000.0
        );
        info!("Total Funding Deficit: {} lamports", total_funding_deficit);
        info!(
            "Accounts with Funding Deficit: {}/{}",
            accounts_with_deficit,
            results.len()
        );
        info!(
            "Accounts with Claim Status: {}/{}",
            accounts_with_claim_status,
            results.len()
        );
        info!(
            "Accounts with Base Reward Claim Status: {}/{}",
            accounts_with_base_reward_claim_status,
            results.len()
        );

        // Print funding deficit transfer commands for epoch 808
        info!("");
        info!("=== Funding Deficit Transfer Commands for Epoch 808 ===");
        for result in results
            .iter()
            .filter(|r| r.tip_distribution_available_funds > 0)
        {
            println!(
                "{} {:.9} {} {:.9}",
                result.vote_account,
                result.tip_distribution_available_funds as f64 / 1_000_000_000.0,
                result.tip_distribution_account,
                result.mev_revenue as f64 / 1_000_000_000.0,
            );
        }
    }
}

pub async fn analyze_accounts_from_json(
    rpc_client: Arc<RpcClient>,
    tip_distribution_program_id: Pubkey,
    tip_router_program_id: Pubkey,
    ncn_address: Pubkey,
    epoch: u64,
    json_data: &str,
) -> Result<()> {
    let entries: Vec<AccountAnalysisEntry> = serde_json::from_str(json_data)?;

    let analyzer = AccountAnalyzer::new(
        rpc_client,
        tip_distribution_program_id,
        tip_router_program_id,
        ncn_address,
        epoch,
    );

    let results = analyzer.analyze_accounts(entries).await?;
    analyzer.print_analysis_results(&results);

    Ok(())
}

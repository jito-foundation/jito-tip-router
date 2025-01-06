use std::str::FromStr;

use crate::{
    args::{Args, ProgramCommand},
    getters::{get_all_operators_in_ncn, get_all_vaults_in_ncn, get_ncn, get_ncn_operator_state},
    instructions::{create_and_add_test_operator, create_test_ncn},
};
use anyhow::{anyhow, Result};
use log::info;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};

pub struct CliHandler {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
    keypair: Option<Keypair>,
    pub restaking_program_id: Pubkey,
    pub vault_program_id: Pubkey,
    pub tip_router_program_id: Pubkey,
    pub tip_distribution_program_id: Pubkey,
    ncn: Option<Pubkey>,
    pub epoch: u64,
}

impl CliHandler {
    pub async fn from_args(args: &Args) -> Result<Self> {
        let rpc_url = args.rpc_url.clone();
        CommitmentConfig::confirmed();
        let commitment = CommitmentConfig::from_str(&args.commitment)?;

        let keypair = args
            .keypair_path
            .as_ref()
            .map(|k| read_keypair_file(k).unwrap());

        let restaking_program_id = Pubkey::from_str(&args.restaking_program_id)?;

        let vault_program_id = Pubkey::from_str(&args.vault_program_id)?;

        let tip_router_program_id = Pubkey::from_str(&args.tip_router_program_id)?;

        let tip_distribution_program_id = Pubkey::from_str(&args.tip_distribution_program_id)?;

        let ncn = args
            .ncn
            .clone()
            .map(|id| Pubkey::from_str(&id))
            .transpose()?;

        let mut handler = Self {
            rpc_url,
            commitment,
            keypair,
            restaking_program_id,
            vault_program_id,
            tip_router_program_id,
            tip_distribution_program_id,
            ncn,
            epoch: u64::MAX,
        };

        handler.epoch = {
            if args.epoch.is_some() {
                args.epoch.unwrap()
            } else {
                let client = handler.rpc_client();
                let epoch_info = client.get_epoch_info().await?;
                epoch_info.epoch
            }
        };

        Ok(handler)
    }

    pub fn rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(self.rpc_url.clone(), self.commitment)
    }

    pub fn keypair(&self) -> Result<&Keypair> {
        self.keypair.as_ref().ok_or_else(|| anyhow!("No keypair"))
    }

    pub fn ncn(&self) -> Result<&Pubkey> {
        self.ncn.as_ref().ok_or_else(|| anyhow!("No NCN address"))
    }

    pub async fn handle(&self, action: ProgramCommand) -> Result<()> {
        match action {
            ProgramCommand::Test {} => self.test().await,
            ProgramCommand::CreateTestNcn {} => self.create_test_ncn().await,
            ProgramCommand::CreateAndAddTestOperator { operator_fee_bps } => {
                self.create_and_add_test_operator(operator_fee_bps).await
            }
            ProgramCommand::GetNcn {} => self.get_ncn().await,
            ProgramCommand::GetNcnOperatorState { operator } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator arg");
                self.get_ncn_operator_state(&operator).await
            }
            ProgramCommand::GetAllOperatorsInNcn {} => self.get_all_operators_in_ncn().await,
            ProgramCommand::GetAllVaultsInNcn {} => self.get_all_vaults_in_ncn().await,
        }
    }

    // --------------- HELPERS -----------------
    async fn test(&self) -> Result<()> {
        info!("Test! {}", self.tip_router_program_id);
        Ok(())
    }

    async fn create_test_ncn(&self) -> Result<()> {
        info!("Creating Test NCN...");
        create_test_ncn(self).await?;
        Ok(())
    }

    async fn create_and_add_test_operator(&self, operator_fee_bps: u16) -> Result<()> {
        info!("Creating and adding operator for {}...", self.ncn()?);
        create_and_add_test_operator(self, operator_fee_bps).await?;
        Ok(())
    }

    // --------------- GETTERS -----------------
    async fn get_ncn(&self) -> Result<()> {
        info!("Getting NCN...");
        let ncn = get_ncn(self).await?;

        info!("NCN: {:?}", ncn);
        Ok(())
    }

    async fn get_ncn_operator_state(&self, operator: &Pubkey) -> Result<()> {
        info!("Getting NCN Operator State for {}...", operator);
        let ncn_operator_state = get_ncn_operator_state(self, operator).await?;
        info!("NCN Operator State: {:?}", ncn_operator_state);
        Ok(())
    }

    async fn get_all_operators_in_ncn(&self) -> Result<()> {
        info!("Getting all operators in NCN...");
        let operators = get_all_operators_in_ncn(self).await?;
        info!("Operators: {:?}", operators.len());
        Ok(())
    }

    async fn get_all_vaults_in_ncn(&self) -> Result<()> {
        info!("Getting all vaults in NCN...");
        let vaults = get_all_vaults_in_ncn(self).await?;
        info!("Vaults: {:?}", vaults.len());
        Ok(())
    }
}

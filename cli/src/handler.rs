use std::str::FromStr;

use crate::args::{Args, ProgramCommand};
use anyhow::{anyhow, Result};
use log::{debug, error, info};
use solana_rpc_client::rpc_client::RpcClient;
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
                let epoch_info = client.get_epoch_info()?;
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
        }
    }

    async fn test(&self) -> Result<()> {
        info!("Test!");

        Ok(())
    }
}

// pub struct CliHandler {
//     cli_config: CliConfig,
//     restaking_program_id: Pubkey,
//     vault_program_id: Pubkey,
//     restaking_program_id: Pubkey,
// }

// impl CliHandler {
//     pub const fn new(
//         cli_config: CliConfig,
//         restaking_program_id: Pubkey,
//         vault_program_id: Pubkey,
//         restaking_program_id: Pubkey,
//     ) -> Self {
//         Self {
//             cli_config,
//             restaking_program_id,
//             vault_program_id,
//             restaking_program_id,
//         }
//     }

//     fn get_rpc_client(&self) -> RpcClient {
//         RpcClient::new_with_commitment(self.cli_config.rpc_url.clone(), self.cli_config.commitment)
//     }

//     pub async fn handle(&self, action: ProgramCommand) -> Result<()> {
//         match action {
//             ProgramCommand::Test {} => self.test().await,
//         }
//     }

//     async fn test(&self) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("No keypair"))?;
//         let rpc_client = self.get_rpc_client();

//         let config_address = Config::find_program_address(&self.restaking_program_id).0;
//         let mut ix_builder = InitializeConfigBuilder::new();
//         ix_builder
//             .config(config_address)
//             .admin(keypair.pubkey())
//             .vault_program(self.vault_program_id);
//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair],
//             blockhash,
//         );
//         info!("Initializing restaking config parameters: {:?}", ix_builder);
//         info!(
//             "Initializing restaking config transaction: {:?}",
//             tx.get_signature()
//         );
//         rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed: {:?}", tx.get_signature());
//         Ok(())
//     }

//     pub async fn initialize_ncn(&self) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("No keypair"))?;
//         let rpc_client = self.get_rpc_client();

//         let base = Keypair::new();
//         let ncn = Ncn::find_program_address(&self.restaking_program_id, &base.pubkey()).0;

//         let mut ix_builder = InitializeNcnBuilder::new();
//         ix_builder
//             .config(Config::find_program_address(&self.restaking_program_id).0)
//             .ncn(ncn)
//             .admin(keypair.pubkey())
//             .base(base.pubkey())
//             .instruction();

//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair, &base],
//             blockhash,
//         );
//         info!("Initializing NCN: {:?}", ncn);
//         info!("Initializing NCN transaction: {:?}", tx.get_signature());
//         let result = rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed: {:?}", result);
//         let statuses = rpc_client
//             .get_signature_statuses(&[*tx.get_signature()])
//             .await?;

//         let tx_status = statuses
//             .value
//             .first()
//             .unwrap()
//             .as_ref()
//             .ok_or_else(|| anyhow!("No signature status"))?;
//         info!("Transaction status: {:?}", tx_status);
//         info!("NCN initialized at address: {:?}", ncn);

//         Ok(())
//     }

//     pub async fn initialize_operator(&self, operator_fee_bps: u16) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("No keypair"))?;
//         let rpc_client = self.get_rpc_client();

//         let base = Keypair::new();
//         let operator = Operator::find_program_address(&self.restaking_program_id, &base.pubkey()).0;

//         let mut ix_builder = InitializeOperatorBuilder::new();
//         ix_builder
//             .config(Config::find_program_address(&self.restaking_program_id).0)
//             .operator(operator)
//             .admin(keypair.pubkey())
//             .base(base.pubkey())
//             .operator_fee_bps(operator_fee_bps)
//             .instruction();

//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair, &base],
//             blockhash,
//         );
//         info!("Initializing operator: {:?}", operator);
//         info!(
//             "Initializing operator transaction: {:?}",
//             tx.get_signature()
//         );
//         rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed");
//         let statuses = rpc_client
//             .get_signature_statuses(&[*tx.get_signature()])
//             .await?;

//         let tx_status = statuses
//             .value
//             .first()
//             .unwrap()
//             .as_ref()
//             .ok_or_else(|| anyhow!("No signature status"))?;
//         info!("Transaction status: {:?}", tx_status);
//         info!("Operator initialized at address: {:?}", operator);

//         Ok(())
//     }

//     pub async fn initialize_operator_vault_ticket(
//         &self,
//         operator: String,
//         vault: String,
//     ) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("Keypair not provided"))?;
//         let rpc_client = self.get_rpc_client();

//         let operator = Pubkey::from_str(&operator)?;
//         let vault = Pubkey::from_str(&vault)?;

//         let operator_vault_ticket = OperatorVaultTicket::find_program_address(
//             &self.restaking_program_id,
//             &operator,
//             &vault,
//         )
//         .0;

//         let mut ix_builder = InitializeOperatorVaultTicketBuilder::new();
//         ix_builder
//             .config(Config::find_program_address(&self.restaking_program_id).0)
//             .operator(operator)
//             .vault(vault)
//             .admin(keypair.pubkey())
//             .operator_vault_ticket(operator_vault_ticket)
//             .payer(keypair.pubkey());

//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair],
//             blockhash,
//         );

//         info!(
//             "Initializing operator vault ticket transaction: {:?}",
//             tx.get_signature()
//         );
//         let result = rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed: {:?}", result);

//         info!("\nCreated Operator Vault Ticket");
//         info!("Operator address: {}", operator);
//         info!("Vault address: {}", vault);
//         info!("Operator Vault Ticket address: {}", operator_vault_ticket);

//         Ok(())
//     }

//     pub async fn warmup_operator_vault_ticket(
//         &self,
//         operator: String,
//         vault: String,
//     ) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("Keypair not provided"))?;
//         let rpc_client = self.get_rpc_client();

//         let operator = Pubkey::from_str(&operator)?;
//         let vault = Pubkey::from_str(&vault)?;

//         let operator_vault_ticket = OperatorVaultTicket::find_program_address(
//             &self.restaking_program_id,
//             &operator,
//             &vault,
//         )
//         .0;

//         let mut ix_builder = WarmupOperatorVaultTicketBuilder::new();
//         ix_builder
//             .config(Config::find_program_address(&self.restaking_program_id).0)
//             .operator(operator)
//             .vault(vault)
//             .operator_vault_ticket(operator_vault_ticket)
//             .admin(keypair.pubkey());

//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair],
//             blockhash,
//         );

//         info!(
//             "Warming up operator vault ticket transaction: {:?}",
//             tx.get_signature()
//         );
//         let result = rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed: {:?}", result);

//         Ok(())
//     }

//     pub async fn get_config(&self) -> Result<()> {
//         let rpc_client = self.get_rpc_client();

//         let config_address = Config::find_program_address(&self.restaking_program_id).0;
//         debug!(
//             "Reading the restaking configuration account at address: {}",
//             config_address
//         );

//         let account = rpc_client.get_account(&config_address).await?;
//         let config = Config::try_from_slice_unchecked(&account.data)?;
//         info!(
//             "Restaking config at address {}: {:?}",
//             config_address, config
//         );
//         Ok(())
//     }

//     pub async fn get_ncn(&self, pubkey: String) -> Result<()> {
//         let pubkey = Pubkey::from_str(&pubkey)?;
//         let account = self.get_rpc_client().get_account(&pubkey).await?;
//         let ncn = Ncn::try_from_slice_unchecked(&account.data)?;
//         info!("NCN at address {}: {:?}", pubkey, ncn);
//         Ok(())
//     }

//     pub async fn list_ncn(&self) -> Result<()> {
//         let rpc_client = self.get_rpc_client();
//         let accounts = rpc_client
//             .get_program_accounts_with_config(
//                 &self.restaking_program_id,
//                 RpcProgramAccountsConfig {
//                     filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
//                         0,
//                         MemcmpEncodedBytes::Bytes(vec![Ncn::DISCRIMINATOR]),
//                     ))]),
//                     account_config: RpcAccountInfoConfig {
//                         encoding: Some(UiAccountEncoding::Base64),
//                         data_slice: None,
//                         commitment: None,
//                         min_context_slot: None,
//                     },
//                     with_context: None,
//                 },
//             )
//             .await?;
//         for (ncn_pubkey, ncn) in accounts {
//             let ncn = Ncn::try_from_slice_unchecked(&ncn.data)?;
//             info!("NCN at address {}: {:?}", ncn_pubkey, ncn);
//         }
//         Ok(())
//     }

//     pub async fn get_operator(&self, pubkey: String) -> Result<()> {
//         let pubkey = Pubkey::from_str(&pubkey)?;
//         let account = self.get_rpc_client().get_account(&pubkey).await?;
//         let operator = Operator::try_from_slice_unchecked(&account.data)?;
//         info!("Operator at address {}: {:?}", pubkey, operator);

//         Ok(())
//     }

//     pub async fn list_operator(&self) -> Result<()> {
//         let rpc_client = self.get_rpc_client();
//         let accounts = rpc_client
//             .get_program_accounts_with_config(
//                 &self.restaking_program_id,
//                 RpcProgramAccountsConfig {
//                     filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new(
//                         0,
//                         MemcmpEncodedBytes::Bytes(vec![Operator::DISCRIMINATOR]),
//                     ))]),
//                     account_config: RpcAccountInfoConfig {
//                         encoding: Some(UiAccountEncoding::Base64),
//                         data_slice: None,
//                         commitment: None,
//                         min_context_slot: None,
//                     },
//                     with_context: None,
//                 },
//             )
//             .await?;
//         for (operator_pubkey, operator) in accounts {
//             let operator = Operator::try_from_slice_unchecked(&operator.data)?;
//             info!("Operator at address {}: {:?}", operator_pubkey, operator);
//         }
//         Ok(())
//     }

//     async fn set_config_admin(&self, new_admin: Pubkey) -> Result<()> {
//         let keypair = self
//             .cli_config
//             .keypair
//             .as_ref()
//             .ok_or_else(|| anyhow!("No keypair"))?;
//         let rpc_client = self.get_rpc_client();

//         let config_address = Config::find_program_address(&self.restaking_program_id).0;
//         let mut ix_builder = SetConfigAdminBuilder::new();
//         ix_builder
//             .config(config_address)
//             .old_admin(keypair.pubkey())
//             .new_admin(new_admin);

//         let blockhash = rpc_client.get_latest_blockhash().await?;
//         let tx = Transaction::new_signed_with_payer(
//             &[ix_builder.instruction()],
//             Some(&keypair.pubkey()),
//             &[keypair],
//             blockhash,
//         );
//         info!(
//             "Setting restaking config admin parameters: {:?}",
//             ix_builder
//         );
//         info!(
//             "Setting restaking config admin transaction: {:?}",
//             tx.get_signature()
//         );
//         rpc_client.send_and_confirm_transaction(&tx).await?;
//         info!("Transaction confirmed: {:?}", tx.get_signature());
//         Ok(())
//     }
// }

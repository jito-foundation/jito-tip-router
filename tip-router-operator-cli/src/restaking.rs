use std::sync::Arc;

use base64::{engine::general_purpose, Engine};
use jito_bytemuck::{AccountDeserialize, Discriminator};
use jito_restaking_client::instructions::{
    InitializeOperatorVaultTicketBuilder, OperatorWarmupNcnBuilder,
};
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator_vault_ticket::OperatorVaultTicket,
};
use log::{info, warn};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{pubkey::Pubkey, signature::Keypair, signer::Signer, transaction::Transaction};

/// Handles jito-restaking-program operation
pub struct RestakingHandler {
    /// RPC Client
    rpc_client: Arc<RpcClient>,

    /// Jito Restaking program ID
    restaking_program_id: Pubkey,

    /// Jito Restaking wonfiguration public key
    restaking_config_address: Pubkey,

    /// NCN address
    ncn_address: Pubkey,

    /// Operator address
    operator_address: Pubkey,

    /// Keypair
    keypair: Arc<Keypair>,
}

impl RestakingHandler {
    /// Initialize [`RestakingHandler`]
    pub const fn new(
        rpc_client: Arc<RpcClient>,
        restaking_program_id: Pubkey,
        restaking_config_address: Pubkey,
        ncn_address: Pubkey,
        operator_address: Pubkey,
        keypair: Arc<Keypair>,
    ) -> Self {
        Self {
            rpc_client,
            restaking_program_id,
            restaking_config_address,
            ncn_address,
            operator_address,
            keypair,
        }
    }

    /// Warmup NCN <> Operator state
    ///
    /// # Process
    ///
    /// - Check if there is an [`NcnOperatorState`] state and loudly warn (but not fail) if there is not existing
    /// - If the [`NcnOperatorState`] ticket exists and the operator is not warmed up, execute that instruction (`operator_warmup_ncn`)
    pub async fn warmup_operator(&self) -> anyhow::Result<()> {
        let slot = self.rpc_client.get_slot().await?;
        let restaking_config_acc = self
            .rpc_client
            .get_account(&self.restaking_config_address)
            .await?;
        let restaking_config =
            RestakingConfig::try_from_slice_unchecked(&restaking_config_acc.data)?;

        let ncn_operator_state_addr = NcnOperatorState::find_program_address(
            &self.restaking_program_id,
            &self.ncn_address,
            &self.operator_address,
        )
        .0;
        match self.rpc_client.get_account(&ncn_operator_state_addr).await {
            Ok(account) => {
                let ncn_operator_state = NcnOperatorState::try_from_slice_unchecked(&account.data)?;
                if !ncn_operator_state
                    .operator_opt_in_state
                    .is_active(slot, restaking_config.epoch_length())?
                {
                    let mut ix_builder = OperatorWarmupNcnBuilder::new();
                    ix_builder
                        .config(self.restaking_config_address)
                        .ncn(self.ncn_address)
                        .operator(self.operator_address)
                        .ncn_operator_state(ncn_operator_state_addr)
                        .admin(self.keypair.pubkey());
                    let mut ix = ix_builder.instruction();
                    ix.program_id = self.restaking_program_id;

                    let blockhash = self.rpc_client.get_latest_blockhash().await?;
                    let tx = Transaction::new_signed_with_payer(
                        &[ix],
                        Some(&self.keypair.pubkey()),
                        &[self.keypair.clone()],
                        blockhash,
                    );
                    let result = self.rpc_client.send_and_confirm_transaction(&tx).await?;

                    info!("Transaction confirmed: {result:?}");
                }
            }
            Err(e) => warn!("Failed to find NcnOperatorState, Please contact NCN admin!: {e}"),
        }

        Ok(())
    }

    /// Create Operator <> Vault tickets
    ///
    /// # Process
    ///
    /// - For all vaults that have [`NcnVaultTicket`] with the NCN address, build and execute the instructions to create [`OperatorVaultTicket`] tickets, if they have not been created already.
    pub async fn create_operator_vault_tickets(&self) -> anyhow::Result<()> {
        let config = {
            let data_size = std::mem::size_of::<NcnVaultTicket>()
                .checked_add(8)
                .ok_or_else(|| anyhow::anyhow!("Failed to add"))?
                .checked_add(32)
                .ok_or_else(|| anyhow::anyhow!("Failed to add"))?;
            let mut slice = Vec::new();
            slice.extend(vec![NcnVaultTicket::DISCRIMINATOR, 0, 0, 0, 0, 0, 0, 0]);
            slice.extend_from_slice(self.ncn_address.as_array());
            let encoded_slice = general_purpose::STANDARD.encode(slice);
            let memcmp =
                RpcFilterType::Memcmp(Memcmp::new(0, MemcmpEncodedBytes::Base64(encoded_slice)));
            RpcProgramAccountsConfig {
                filters: Some(vec![RpcFilterType::DataSize(data_size as u64), memcmp]),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    data_slice: Some(UiDataSliceConfig {
                        offset: 0,
                        length: data_size,
                    }),
                    commitment: None,
                    min_context_slot: None,
                },
                with_context: Some(false),
                sort_results: Some(false),
            }
        };
        let ncn_vault_tickets = self
            .rpc_client
            .get_program_accounts_with_config(&self.restaking_program_id, config)
            .await?;

        for (_ncn_vault_ticket_addr, ncn_vault_ticket_acc) in ncn_vault_tickets {
            let keypair = self.keypair.clone();
            let ncn_vault_ticket =
                NcnVaultTicket::try_from_slice_unchecked(&ncn_vault_ticket_acc.data)?;

            let operator_vault_ticket_addr = OperatorVaultTicket::find_program_address(
                &self.restaking_program_id,
                &self.operator_address,
                &ncn_vault_ticket.vault,
            )
            .0;

            match self
                .rpc_client
                .get_account(&operator_vault_ticket_addr)
                .await
            {
                Ok(_account) => {
                    info!(
                        "OperatorVaultTicket already exists for Operator: {}, Vault: {}",
                        self.operator_address, ncn_vault_ticket.vault
                    );
                    continue;
                }
                Err(_e) => {
                    let mut ix_builder = InitializeOperatorVaultTicketBuilder::new();
                    ix_builder
                        .config(self.restaking_config_address)
                        .operator(self.operator_address)
                        .vault(ncn_vault_ticket.vault)
                        .admin(keypair.pubkey())
                        .operator_vault_ticket(operator_vault_ticket_addr)
                        .payer(self.keypair.pubkey());
                    let mut ix = ix_builder.instruction();
                    ix.program_id = self.restaking_program_id;

                    let blockhash = self.rpc_client.get_latest_blockhash().await?;
                    let tx = Transaction::new_signed_with_payer(
                        &[ix],
                        Some(&self.keypair.pubkey()),
                        &[keypair],
                        blockhash,
                    );
                    let result = self.rpc_client.send_and_confirm_transaction(&tx).await?;

                    info!("Transaction confirmed: {:?}", result);
                    info!(
                        "Created OperatorVault Ticket, Operator: {}, Vault: {}",
                        self.operator_address, ncn_vault_ticket.vault
                    );
                }
            }
        }

        Ok(())
    }
}

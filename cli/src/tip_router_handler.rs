use anyhow::anyhow;
use jito_bytemuck::AccountDeserialize;
use jito_tip_router_client::instructions::InitializeConfigBuilder;
use log::{debug, info};
use solana_rpc_client::{nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction};
use solana_sdk::{pubkey::Pubkey, signer::Signer, transaction::Transaction};

use crate::{
    tip_router::{ConfigActions, TipRouterCommands},
    CliConfig,
};

pub struct TipRouterCliHandler {
    cli_config: CliConfig,

    restaking_program_id: Pubkey,

    tip_router_program_id: Pubkey,
}

impl TipRouterCliHandler {
    pub const fn new(
        cli_config: CliConfig,
        restaking_program_id: Pubkey,
        tip_router_program_id: Pubkey,
    ) -> Self {
        Self {
            cli_config,
            restaking_program_id,
            tip_router_program_id,
        }
    }

    fn get_rpc_client(&self) -> RpcClient {
        RpcClient::new_with_commitment(self.cli_config.rpc_url.clone(), self.cli_config.commitment)
    }

    pub async fn handle(&self, action: TipRouterCommands) -> anyhow::Result<()> {
        match action {
            TipRouterCommands::Config {
                action:
                    ConfigActions::Initialize {
                        ncn,
                        dao_fee_bps,
                        default_ncn_fee_bps,
                        block_engine_fee_bps,
                        epochs_before_stall,
                        valid_slots_after_consensus,
                    },
            } => {
                self.initialize_config(
                    ncn,
                    dao_fee_bps,
                    default_ncn_fee_bps,
                    block_engine_fee_bps,
                    epochs_before_stall,
                    valid_slots_after_consensus,
                )
                .await
            }
            TipRouterCommands::Config {
                action: ConfigActions::Get { ncn },
            } => self.get_config(ncn).await,
            // TipRouterCommands::Config {
            //     action: ConfigActions::SetAdmin { new_admin },
            // } => self.set_config_admin(new_admin).await,
        }
    }

    async fn initialize_config(
        &self,
        ncn: Pubkey,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
        block_engine_fee_bps: u16,
        epochs_before_stall: u64,
        valid_slots_after_consensus: u64,
    ) -> anyhow::Result<()> {
        let keypair = self
            .cli_config
            .keypair
            .as_ref()
            .ok_or_else(|| anyhow!("No keypair"))?;
        let rpc_client = self.get_rpc_client();

        let config_address = jito_tip_router_core::config::Config::find_program_address(
            &self.tip_router_program_id,
            &ncn,
        )
        .0;
        let ncn_admin = keypair.pubkey();
        let mut ix_builder = InitializeConfigBuilder::new();
        ix_builder
            .config(config_address)
            .ncn(ncn)
            .ncn_admin(ncn_admin)
            .fee_wallet(ncn_admin)
            .tie_breaker_admin(ncn_admin)
            .restaking_program(self.restaking_program_id)
            .dao_fee_bps(dao_fee_bps)
            .default_ncn_fee_bps(default_ncn_fee_bps)
            .block_engine_fee_bps(block_engine_fee_bps)
            .epochs_before_stall(epochs_before_stall)
            .valid_slots_after_consensus(valid_slots_after_consensus);
        let mut init_ix = ix_builder.instruction();
        init_ix.program_id = self.tip_router_program_id;

        let blockhash = rpc_client.get_latest_blockhash().await?;
        let tx =
            Transaction::new_signed_with_payer(&[init_ix], Some(&ncn_admin), &[keypair], blockhash);

        info!(
            "Initializing Jito Tip Router config parameters: {:?}",
            ix_builder
        );
        info!(
            "Initializing Jito Tip Router config transaction: {:?}",
            tx.get_signature()
        );
        rpc_client.send_and_confirm_transaction(&tx).await?;
        info!("Transaction confirmed: {:?}", tx.get_signature());
        Ok(())
    }

    pub async fn get_config(&self, ncn: Pubkey) -> anyhow::Result<()> {
        let rpc_client = self.get_rpc_client();

        let config_address = jito_tip_router_core::config::Config::find_program_address(
            &self.tip_router_program_id,
            &ncn,
        )
        .0;
        debug!(
            "Reading the Jito Tip Router configuration account at address: {}",
            config_address
        );

        let account = rpc_client.get_account(&config_address).await?;
        let config = jito_tip_router_core::config::Config::try_from_slice_unchecked(&account.data)?;
        info!(
            "Jito Tip Router config at address {}: {:?}",
            config_address, config
        );
        Ok(())
    }
}

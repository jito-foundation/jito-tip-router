use std::mem::size_of;

use crate::handler::CliHandler;
use anyhow::{Ok, Result};
use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
};
use jito_tip_router_core::{
    ballot_box::BallotBox,
    base_reward_router::{BaseRewardReceiver, BaseRewardRouter},
    config::Config as TipRouterConfig,
    epoch_snapshot::{EpochSnapshot, OperatorSnapshot},
    epoch_state::EpochState,
    ncn_fee_group::NcnFeeGroup,
    ncn_reward_router::{NcnRewardReceiver, NcnRewardRouter},
    vault_registry::VaultRegistry,
    weight_table::WeightTable,
};
use jito_vault_core::vault::Vault;
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{account::Account, pubkey::Pubkey};

// ---------------------- HELPERS ----------------------
// So we can switch between the two implementations
pub async fn get_account(handler: &CliHandler, account: &Pubkey) -> Result<Account> {
    let client = handler.rpc_client();
    let account = client.get_account(account).await?;
    Ok(account)
}

// ---------------------- TIP ROUTER ----------------------
pub async fn get_tip_router_config(handler: &CliHandler) -> Result<TipRouterConfig> {
    let (address, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;
    let account = TipRouterConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_registry(handler: &CliHandler) -> Result<VaultRegistry> {
    let (address, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;
    let account = VaultRegistry::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_weight_table(handler: &CliHandler) -> Result<WeightTable> {
    let (address, _, _) = WeightTable::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = WeightTable::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_snapshot(handler: &CliHandler) -> Result<EpochSnapshot> {
    let (address, _, _) = EpochSnapshot::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = EpochSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
) -> Result<OperatorSnapshot> {
    let (address, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = OperatorSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ballot_box(handler: &CliHandler) -> Result<BallotBox> {
    let (address, _, _) = BallotBox::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = BallotBox::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_router(handler: &CliHandler) -> Result<BaseRewardRouter> {
    let (address, _, _) = BaseRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = BaseRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_receiver(handler: &CliHandler) -> Result<Account> {
    let (address, _, _) = BaseRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    Ok(account)
}

pub async fn get_ncn_reward_router(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
) -> Result<NcnRewardRouter> {
    let (address, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    let account = NcnRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_reward_reciever(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
) -> Result<Account> {
    let (address, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        handler.epoch,
    );

    let account = get_account(handler, &address).await?;
    Ok(account)
}

// ---------------------- RESTAKING ----------------------

pub async fn get_restaking_config(handler: &CliHandler) -> Result<RestakingConfig> {
    let (address, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);
    let account = get_account(handler, &address).await?;
    let account = RestakingConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn(handler: &CliHandler) -> Result<Ncn> {
    let account = get_account(handler, handler.ncn()?).await?;
    let account = Ncn::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault(handler: &CliHandler, vault: &Pubkey) -> Result<Vault> {
    let account = get_account(handler, vault).await?;
    let account = Vault::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator(handler: &CliHandler, operator: &Pubkey) -> Result<Operator> {
    let account = get_account(handler, operator).await?;
    let account = Operator::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_operator_state(
    handler: &CliHandler,
    operator: &Pubkey,
) -> Result<NcnOperatorState> {
    let (address, _, _) = NcnOperatorState::find_program_address(
        &handler.restaking_program_id,
        handler.ncn()?,
        operator,
    );

    let account = get_account(handler, &address).await?;
    let account = NcnOperatorState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_all_operators_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();

    let ncn_operator_state_size = size_of::<NcnOperatorState>() + 8;

    let size_filter = RpcFilterType::DataSize(ncn_operator_state_size as u64);

    let ncn_filter = RpcFilterType::Memcmp(Memcmp::new(
        8,                                                           // offset
        MemcmpEncodedBytes::Bytes(handler.ncn()?.to_bytes().into()), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, ncn_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: ncn_operator_state_size,
            }),
            commitment: Some(handler.commitment),
            min_context_slot: None,
        },
        with_context: Some(false),
    };

    let results = client
        .get_program_accounts_with_config(&handler.restaking_program_id, config)
        .await?;

    let accounts: Vec<(Pubkey, NcnOperatorState)> = results
        .iter()
        .filter_map(|result| {
            NcnOperatorState::try_from_slice_unchecked(result.1.data.as_slice())
                .map(|account| (result.0, *account))
                .ok()
        })
        .collect();

    let operators = accounts
        .iter()
        .map(|(_, ncn_operator_state)| ncn_operator_state.operator)
        .collect::<Vec<Pubkey>>();

    Ok(operators)
}

pub async fn get_all_vaults_in_ncn(handler: &CliHandler) -> Result<Vec<Pubkey>> {
    let client = handler.rpc_client();

    let ncn_vault_ticket_size = size_of::<NcnVaultTicket>() + 8;

    let size_filter = RpcFilterType::DataSize(ncn_vault_ticket_size as u64);

    let ncn_filter = RpcFilterType::Memcmp(Memcmp::new(
        8,                                                           // offset
        MemcmpEncodedBytes::Bytes(handler.ncn()?.to_bytes().into()), // encoded bytes
    ));

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![size_filter, ncn_filter]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: Some(UiDataSliceConfig {
                offset: 0,
                length: ncn_vault_ticket_size,
            }),
            commitment: Some(handler.commitment),
            min_context_slot: None,
        },
        with_context: Some(false),
    };

    let results = client
        .get_program_accounts_with_config(&handler.restaking_program_id, config)
        .await?;

    let accounts: Vec<(Pubkey, NcnVaultTicket)> = results
        .iter()
        .filter_map(|result| {
            NcnVaultTicket::try_from_slice_unchecked(result.1.data.as_slice())
                .map(|account| (result.0, *account))
                .ok()
        })
        .collect();

    let vaults = accounts
        .iter()
        .map(|(_, ncn_operator_state)| ncn_operator_state.vault)
        .collect::<Vec<Pubkey>>();

    Ok(vaults)
}

#[derive(Default)]
pub struct TipRouterEpochState {
    pub ncn: Pubkey,
    pub vaults: Vec<Pubkey>,
    pub operators: Vec<Pubkey>,
    pub tip_router_config_address: Pubkey,
    pub vault_registry_address: Pubkey,
    pub epoch_state_address: Pubkey,
    pub weight_table_address: Pubkey,
    pub epoch_snapshot_address: Pubkey,
    pub operator_snapshots_address: Vec<Pubkey>,
    pub ballot_box_address: Pubkey,
    pub base_reward_router_address: Pubkey,
    pub base_reward_receiver_address: Pubkey,
    pub ncn_reward_routers_address: Vec<Vec<Pubkey>>,
    pub ncn_reward_receivers_address: Vec<Vec<Pubkey>>,
}

impl TipRouterEpochState {
    pub async fn fetch(handler: &CliHandler) -> Self {
        let epoch = handler.epoch;

        let mut state: Self = Self::default();

        // Fetch all vaults and operators
        let ncn = *handler.ncn().unwrap();
        state.ncn = ncn;

        let vaults = get_all_vaults_in_ncn(handler).await.unwrap();
        state.vaults = vaults;

        let operators = get_all_operators_in_ncn(handler).await.unwrap();
        state.operators = operators;

        let (tip_router_config_address, _, _) =
            TipRouterConfig::find_program_address(&handler.tip_router_program_id, &ncn);
        state.tip_router_config_address = tip_router_config_address;

        let (vault_registry_address, _, _) =
            VaultRegistry::find_program_address(&handler.tip_router_program_id, &ncn);
        state.vault_registry_address = vault_registry_address;

        let (epoch_state_address, _, _) =
            WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.epoch_state_address = epoch_state_address;

        let (weight_table_address, _, _) =
            WeightTable::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.weight_table_address = weight_table_address;

        let (epoch_snapshot_address, _, _) =
            EpochSnapshot::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.epoch_snapshot_address = epoch_snapshot_address;

        for operator in state.operators.iter() {
            let (operator_snapshot_address, _, _) = OperatorSnapshot::find_program_address(
                &handler.tip_router_program_id,
                operator,
                &ncn,
                epoch,
            );
            state
                .operator_snapshots_address
                .push(operator_snapshot_address);
        }

        let (ballot_box_address, _, _) =
            BallotBox::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.ballot_box_address = ballot_box_address;

        let (base_reward_router_address, _, _) =
            BaseRewardRouter::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.base_reward_router_address = base_reward_router_address;

        let (base_reward_receiver_address, _, _) =
            BaseRewardReceiver::find_program_address(&handler.tip_router_program_id, &ncn, epoch);
        state.base_reward_receiver_address = base_reward_receiver_address;

        for operator in state.operators.iter() {
            let mut ncn_reward_routers_address = Vec::default();
            let mut ncn_reward_receivers_address = Vec::default();

            for ncn_fee_group in NcnFeeGroup::all_groups() {
                let (ncn_reward_router_address, _, _) = NcnRewardRouter::find_program_address(
                    &handler.tip_router_program_id,
                    ncn_fee_group,
                    operator,
                    &ncn,
                    epoch,
                );
                ncn_reward_routers_address.push(ncn_reward_router_address);

                let (ncn_reward_receiver_address, _, _) = NcnRewardReceiver::find_program_address(
                    &handler.tip_router_program_id,
                    ncn_fee_group,
                    operator,
                    &ncn,
                    epoch,
                );
                ncn_reward_receivers_address.push(ncn_reward_receiver_address);
            }

            state
                .ncn_reward_routers_address
                .push(ncn_reward_routers_address);
            state
                .ncn_reward_receivers_address
                .push(ncn_reward_receivers_address);
        }

        todo!();
    }

    pub async fn tip_router_config(&self, handler: &CliHandler) -> Result<Option<TipRouterConfig>> {
        let raw_account = get_account(handler, &self.tip_router_config_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = TipRouterConfig::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn vault_registry(&self, handler: &CliHandler) -> Result<Option<VaultRegistry>> {
        let raw_account = get_account(handler, &self.vault_registry_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = VaultRegistry::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn epoch_state(&self, handler: &CliHandler) -> Result<Option<Box<EpochState>>> {
        let raw_account = get_account(handler, &self.epoch_state_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = Box::new(*EpochState::try_from_slice_unchecked(
                raw_account.data.as_slice(),
            )?);
            Ok(Some(account))
        }
    }

    pub async fn weight_table(&self, handler: &CliHandler) -> Result<Option<WeightTable>> {
        let raw_account = get_account(handler, &self.weight_table_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = WeightTable::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn epoch_snapshot(&self, handler: &CliHandler) -> Result<Option<EpochSnapshot>> {
        let raw_account = get_account(handler, &self.epoch_snapshot_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = EpochSnapshot::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn operator_snapshot(
        &self,
        handler: &CliHandler,
        operator_index: usize,
    ) -> Result<Option<OperatorSnapshot>> {
        let raw_account =
            get_account(handler, &self.operator_snapshots_address[operator_index]).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = OperatorSnapshot::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn ballot_box(&self, handler: &CliHandler) -> Result<Option<Box<BallotBox>>> {
        let raw_account = get_account(handler, &self.ballot_box_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = Box::new(*BallotBox::try_from_slice_unchecked(
                raw_account.data.as_slice(),
            )?);
            Ok(Some(account))
        }
    }

    pub async fn base_reward_router(
        &self,
        handler: &CliHandler,
    ) -> Result<Option<BaseRewardRouter>> {
        let raw_account = get_account(handler, &self.base_reward_router_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = BaseRewardRouter::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn base_reward_receiver(&self, handler: &CliHandler) -> Result<Option<Account>> {
        let raw_account = get_account(handler, &self.base_reward_receiver_address).await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(raw_account))
        }
    }

    pub async fn ncn_reward_router(
        &self,
        handler: &CliHandler,
        operator_index: usize,
        ncn_fee_group: NcnFeeGroup,
    ) -> Result<Option<NcnRewardRouter>> {
        let raw_account = get_account(
            handler,
            &self.ncn_reward_routers_address[operator_index][ncn_fee_group.group_index()?],
        )
        .await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            let account = NcnRewardRouter::try_from_slice_unchecked(raw_account.data.as_slice())?;
            Ok(Some(*account))
        }
    }

    pub async fn ncn_reward_receiver(
        &self,
        handler: &CliHandler,
        operator_index: usize,
        ncn_fee_group: NcnFeeGroup,
    ) -> Result<Option<Account>> {
        let raw_account = get_account(
            handler,
            &self.ncn_reward_receivers_address[operator_index][ncn_fee_group.group_index()?],
        )
        .await?;

        if raw_account.data.is_empty() {
            Ok(None)
        } else {
            Ok(Some(raw_account))
        }
    }

    pub async fn get_state(&self, handler: &CliHandler) -> Result<TipRouterState> {
        let tip_router_config = self.tip_router_config(handler).await?;
        let vault_registry = self.vault_registry(handler).await?;

        if tip_router_config.is_none() || vault_registry.is_none() {
            return Ok(TipRouterState::NotConfigured);
        }

        let weight_table = self.weight_table(handler).await?;

        if weight_table.is_none() {
            return Ok(TipRouterState::Idle);
        }

        // let epoch_snapshot = self.epoch_snapshot(handler).await?;

        todo!()
    }
}

pub enum TipRouterState {
    NotConfigured,
    Idle,
    Snapshotting,
    Voting,
    Routing,
}

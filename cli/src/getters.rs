use std::fmt;
use std::mem::size_of;

use crate::handler::CliHandler;
use anyhow::Result;
use jito_bytemuck::AccountDeserialize;
use jito_restaking_core::{
    config::Config as RestakingConfig, ncn::Ncn, ncn_operator_state::NcnOperatorState,
    ncn_vault_ticket::NcnVaultTicket, operator::Operator,
    operator_vault_ticket::OperatorVaultTicket,
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
use jito_vault_core::{
    vault::Vault, vault_ncn_ticket::VaultNcnTicket,
    vault_operator_delegation::VaultOperatorDelegation,
};
use solana_account_decoder::{UiAccountEncoding, UiDataSliceConfig};
use solana_client::{
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_sdk::{account::Account, pubkey::Pubkey};

// ---------------------- HELPERS ----------------------
// So we can switch between the two implementations
pub async fn get_account(handler: &CliHandler, account: &Pubkey) -> Result<Option<Account>> {
    let client = handler.rpc_client();
    let account = client
        .get_account_with_commitment(account, handler.commitment)
        .await?;

    Ok(account.value)
}

// ---------------------- TIP ROUTER ----------------------
pub async fn get_tip_router_config(handler: &CliHandler) -> Result<TipRouterConfig> {
    let (address, _, _) =
        TipRouterConfig::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = TipRouterConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_registry(handler: &CliHandler) -> Result<VaultRegistry> {
    let (address, _, _) =
        VaultRegistry::find_program_address(&handler.tip_router_program_id, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = VaultRegistry::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_state(handler: &CliHandler, epoch: u64) -> Result<EpochState> {
    let (address, _, _) =
        EpochState::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = EpochState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_weight_table(handler: &CliHandler, epoch: u64) -> Result<WeightTable> {
    let (address, _, _) =
        WeightTable::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = WeightTable::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_epoch_snapshot(handler: &CliHandler, epoch: u64) -> Result<EpochSnapshot> {
    let (address, _, _) =
        EpochSnapshot::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = EpochSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_snapshot(
    handler: &CliHandler,
    operator: &Pubkey,
    epoch: u64,
) -> Result<OperatorSnapshot> {
    let (address, _, _) = OperatorSnapshot::find_program_address(
        &handler.tip_router_program_id,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = OperatorSnapshot::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ballot_box(handler: &CliHandler, epoch: u64) -> Result<BallotBox> {
    let (address, _, _) =
        BallotBox::find_program_address(&handler.tip_router_program_id, handler.ncn()?, epoch);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = BallotBox::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_router(handler: &CliHandler, epoch: u64) -> Result<BaseRewardRouter> {
    let (address, _, _) = BaseRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = BaseRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_base_reward_receiver(handler: &CliHandler, epoch: u64) -> Result<Account> {
    let (address, _, _) = BaseRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok(account)
}

pub async fn get_ncn_reward_router(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
    epoch: u64,
) -> Result<NcnRewardRouter> {
    let (address, _, _) = NcnRewardRouter::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnRewardRouter::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_reward_reciever(
    handler: &CliHandler,
    ncn_fee_group: NcnFeeGroup,
    operator: &Pubkey,
    epoch: u64,
) -> Result<Account> {
    let (address, _, _) = NcnRewardReceiver::find_program_address(
        &handler.tip_router_program_id,
        ncn_fee_group,
        operator,
        handler.ncn()?,
        epoch,
    );

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    Ok(account)
}

// ---------------------- RESTAKING ----------------------

pub async fn get_restaking_config(handler: &CliHandler) -> Result<RestakingConfig> {
    let (address, _, _) = RestakingConfig::find_program_address(&handler.restaking_program_id);
    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = RestakingConfig::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn(handler: &CliHandler) -> Result<Ncn> {
    let account = get_account(handler, handler.ncn()?).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = Ncn::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault(handler: &CliHandler, vault: &Pubkey) -> Result<Vault> {
    let account = get_account(handler, vault)
        .await?
        .expect("Account not found");
    let account = Vault::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator(handler: &CliHandler, operator: &Pubkey) -> Result<Operator> {
    let account = get_account(handler, operator).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

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

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnOperatorState::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_ncn_ticket(handler: &CliHandler, vault: &Pubkey) -> Result<VaultNcnTicket> {
    let (address, _, _) =
        VaultNcnTicket::find_program_address(&handler.vault_program_id, vault, handler.ncn()?);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = VaultNcnTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_ncn_vault_ticket(handler: &CliHandler, vault: &Pubkey) -> Result<NcnVaultTicket> {
    let (address, _, _) =
        NcnVaultTicket::find_program_address(&handler.restaking_program_id, handler.ncn()?, vault);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = NcnVaultTicket::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_vault_operator_delegation(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<VaultOperatorDelegation> {
    let (address, _, _) =
        VaultOperatorDelegation::find_program_address(&handler.vault_program_id, vault, operator);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = VaultOperatorDelegation::try_from_slice_unchecked(account.data.as_slice())?;
    Ok(*account)
}

pub async fn get_operator_vault_ticket(
    handler: &CliHandler,
    vault: &Pubkey,
    operator: &Pubkey,
) -> Result<OperatorVaultTicket> {
    let (address, _, _) =
        OperatorVaultTicket::find_program_address(&handler.restaking_program_id, operator, vault);

    let account = get_account(handler, &address).await?;

    if account.is_none() {
        return Err(anyhow::anyhow!("Account not found"));
    }
    let account = account.unwrap();

    let account = OperatorVaultTicket::try_from_slice_unchecked(account.data.as_slice())?;
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

pub async fn get_all_tickets(handler: &CliHandler) -> Result<Vec<NcnTickets>> {
    let client = handler.rpc_client();

    let all_vaults = get_all_vaults_in_ncn(handler).await?;
    let all_operators = get_all_operators_in_ncn(handler).await?;

    let restaking_config = get_restaking_config(handler).await?;

    let slot = client.get_epoch_info().await?.absolute_slot;
    let epoch_length = restaking_config.epoch_length();

    let mut tickets = Vec::new();
    for operator in all_operators.iter() {
        for vault in all_vaults.iter() {
            tickets.push(NcnTickets::fetch(handler, operator, vault, slot, epoch_length).await);
        }
    }

    Ok(tickets)
}

pub struct NcnTickets {
    pub slot: u64,
    pub epoch_length: u64,
    pub ncn: Pubkey,
    pub vault: Pubkey,
    pub operator: Pubkey,
    pub ncn_vault_ticket: Option<NcnVaultTicket>,
    pub vault_ncn_ticket: Option<VaultNcnTicket>,
    pub vault_operator_delegation: Option<VaultOperatorDelegation>,
    pub operator_vault_ticket: Option<OperatorVaultTicket>,
    pub ncn_operator_state: Option<NcnOperatorState>,
}

impl NcnTickets {
    const DNE: u8 = 0;
    const NOT_ACTIVE: u8 = 1;
    const ACTIVE: u8 = 2;

    pub async fn fetch(
        handler: &CliHandler,
        operator: &Pubkey,
        vault: &Pubkey,
        slot: u64,
        epoch_length: u64,
    ) -> Self {
        let ncn = handler.ncn().expect("NCN not found");

        let ncn_vault_ticket = get_ncn_vault_ticket(handler, vault).await;
        let ncn_vault_ticket = {
            match ncn_vault_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        panic!("Error fetching NCN vault ticket: {}", e);
                    }
                }
            }
        };

        let vault_ncn_ticket = get_vault_ncn_ticket(handler, vault).await;
        let vault_ncn_ticket = {
            match vault_ncn_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        panic!("Error fetching NCN vault ticket: {}", e);
                    }
                }
            }
        };

        let vault_operator_delegation =
            get_vault_operator_delegation(handler, vault, operator).await;
        let vault_operator_delegation = {
            match vault_operator_delegation {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        panic!("Error fetching NCN vault ticket: {}", e);
                    }
                }
            }
        };

        let operator_vault_ticket = get_operator_vault_ticket(handler, vault, operator).await;
        let operator_vault_ticket = {
            match operator_vault_ticket {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        panic!("Error fetching NCN vault ticket: {}", e);
                    }
                }
            }
        };

        let ncn_operator_state = get_ncn_operator_state(handler, operator).await;
        let ncn_operator_state = {
            match ncn_operator_state {
                Ok(account) => Some(account),
                Err(e) => {
                    if e.to_string().contains("Account not found") {
                        None
                    } else {
                        panic!("Error fetching NCN vault ticket: {}", e);
                    }
                }
            }
        };

        Self {
            slot,
            epoch_length,
            ncn: *ncn,
            vault: *vault,
            operator: *operator,
            ncn_vault_ticket,
            vault_ncn_ticket,
            vault_operator_delegation,
            operator_vault_ticket,
            ncn_operator_state,
        }
    }

    pub fn ncn_operator(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .ncn_opt_in_state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn operator_ncn(&self) -> u8 {
        if self.ncn_operator_state.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_operator_state
            .as_ref()
            .unwrap()
            .operator_opt_in_state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn ncn_vault(&self) -> u8 {
        if self.ncn_vault_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .ncn_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn vault_ncn(&self) -> u8 {
        if self.vault_ncn_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .vault_ncn_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn operator_vault(&self) -> u8 {
        if self.operator_vault_ticket.is_none() {
            return Self::DNE;
        }

        if self
            .operator_vault_ticket
            .as_ref()
            .unwrap()
            .state
            .is_active(self.slot, self.epoch_length)
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }

    pub fn vault_operator(&self) -> u8 {
        if self.vault_operator_delegation.is_none() {
            return Self::DNE;
        }

        if self
            .vault_operator_delegation
            .as_ref()
            .unwrap()
            .delegation_state
            .total_security()
            .unwrap()
            > 0
        {
            return Self::ACTIVE;
        }

        Self::NOT_ACTIVE
    }
}

impl fmt::Display for NcnTickets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Helper closure for arrow representation
        let arrow = |state: u8| -> &str {
            match state {
                Self::DNE => "--X",
                Self::NOT_ACTIVE => "---",
                Self::ACTIVE => "==>",
                _ => "",
            }
        };

        // Helper closure for checkmarks in summary
        let check = |state: u8| -> &str {
            match state {
                Self::DNE => "🚧",
                Self::NOT_ACTIVE => "❌",
                Self::ACTIVE => "✅",
                _ => "",
            }
        };

        writeln!(f, "\n\n")?;
        writeln!(f, "┌─────────────────────────────────┐")?;
        writeln!(f, "│            State                │")?;
        writeln!(f, "├─────────────────────────────────┤")?;
        writeln!(f, "│                                 │")?;
        writeln!(
            f,
            "│   NCN {}--> Operator           │",
            arrow(self.ncn_operator())
        )?;
        writeln!(
            f,
            "│       <--{}                    │",
            arrow(self.operator_ncn())
        )?;
        writeln!(f, "│                                 │")?;
        writeln!(
            f,
            "│   NCN {}--> Vault              │",
            arrow(self.ncn_vault())
        )?;
        writeln!(
            f,
            "│       <--{}                    │",
            arrow(self.vault_ncn())
        )?;
        writeln!(f, "│                                 │")?;
        writeln!(
            f,
            "│   Operator {}--> Vault         │",
            arrow(self.operator_vault())
        )?;
        writeln!(
            f,
            "│           <--{}                │",
            arrow(self.vault_operator())
        )?;
        writeln!(f, "│                                 │")?;
        writeln!(f, "└─────────────────────────────────┘")?;

        // Summary section
        writeln!(f, "Summary:")?;
        writeln!(
            f,
            "NCN -> Operator: {}      Operator -> NCN: {}",
            check(self.ncn_operator()),
            check(self.operator_ncn())
        )?;
        writeln!(
            f,
            "NCN -> Vault: {}         Vault -> NCN: {}",
            check(self.ncn_vault()),
            check(self.vault_ncn())
        )?;
        writeln!(
            f,
            "Operator -> Vault: {}    Vault -> Operator: {}",
            check(self.operator_vault()),
            check(self.vault_operator())
        )?;
        writeln!(f, "\nncn:      {}", self.ncn)?;
        writeln!(f, "Operator: {}", self.operator)?;
        writeln!(f, "Vault:    {}", self.vault)?;
        writeln!(f, "\n\n")?;

        Ok(())
    }
}

// NCN: HCwK4Hi98Po1PdUzwX8rAmUhvHhmu1LtwBMFaTLFR2TK
// Operator: 7LbsNkrA6RwQ8v76rsg6pdGqwsgHy5Gn3iDkFCwR9Jd8
// Vault: 3WBxbSRwfFEaLAh89VYAVCygmNamkdKiq8Ga2TUJ4FBA
// Amount: 1000
// OVT: 7zRfBmgNFjw7VZLcr52emWxncdgMh55o3xfvu3B2gWwn

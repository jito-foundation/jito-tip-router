#![allow(clippy::integer_division)]
use std::{collections::HashMap, str::FromStr, sync::Arc};

use crate::{
    args::{Args, ProgramCommand},
    getters::{
        get_account_payer, get_all_operators_in_ncn, get_all_tickets, get_all_vaults_in_ncn,
        get_ballot_box, get_base_reward_receiver, get_base_reward_router, get_current_slot,
        get_epoch_snapshot, get_epoch_state, get_is_epoch_completed, get_ncn,
        get_ncn_operator_state, get_ncn_reward_receiver, get_ncn_reward_router,
        get_ncn_vault_ticket, get_operator_snapshot, get_stake_pool, get_tip_router_config,
        get_total_epoch_rent_cost, get_total_rewards_to_be_distributed, get_vault_ncn_ticket,
        get_vault_operator_delegation, get_vault_registry, get_weight_table,
    },
    instructions::{
        admin_create_config, admin_fund_account_payer, admin_register_st_mint,
        admin_set_config_fees, admin_set_new_admin, admin_set_parameters, admin_set_weight,
        crank_switchboard, create_and_add_test_operator, create_and_add_test_vault,
        create_ballot_box, create_base_reward_router, create_epoch_snapshot, create_epoch_state,
        create_ncn_reward_router, create_operator_snapshot, create_test_ncn, create_vault_registry,
        create_weight_table, distribute_base_ncn_rewards, register_vault, route_base_rewards,
        route_ncn_rewards, set_weight, snapshot_vault_operator_delegation,
    },
    keeper::keeper_loop::startup_keeper,
};
use anyhow::{anyhow, Result};
use jito_tip_router_core::{
    account_payer::AccountPayer, base_reward_router::BaseRewardReceiver, ncn_fee_group::NcnFeeGroup,
};
use log::info;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
};
use switchboard_on_demand_client::SbContext;

pub struct CliHandler {
    pub rpc_url: String,
    pub commitment: CommitmentConfig,
    keypair: Option<Keypair>,
    pub restaking_program_id: Pubkey,
    pub vault_program_id: Pubkey,
    pub tip_router_program_id: Pubkey,
    pub tip_distribution_program_id: Pubkey,
    pub token_program_id: Pubkey,
    ncn: Option<Pubkey>,
    pub epoch: u64,
    rpc_client: RpcClient,
    switchboard_context: Arc<SbContext>,
    pub retries: u64,
    pub priority_fee_micro_lamports: u64,
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

        let token_program_id = Pubkey::from_str(&args.token_program_id)?;

        let ncn = args
            .ncn
            .clone()
            .map(|id| Pubkey::from_str(&id))
            .transpose()?;

        let rpc_client = RpcClient::new_with_commitment(rpc_url.clone(), commitment);

        let switchboard_context = SbContext::new();

        let mut handler = Self {
            rpc_url,
            commitment,
            keypair,
            restaking_program_id,
            vault_program_id,
            tip_router_program_id,
            tip_distribution_program_id,
            token_program_id,
            ncn,
            switchboard_context,
            epoch: u64::MAX,
            rpc_client,
            retries: args.transaction_retries,
            priority_fee_micro_lamports: args.priority_fee_micro_lamports,
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

    pub const fn rpc_client(&self) -> &RpcClient {
        &self.rpc_client
    }

    pub const fn switchboard_context(&self) -> &Arc<SbContext> {
        &self.switchboard_context
    }

    pub fn keypair(&self) -> Result<&Keypair> {
        self.keypair.as_ref().ok_or_else(|| anyhow!("No keypair"))
    }

    pub fn ncn(&self) -> Result<&Pubkey> {
        self.ncn.as_ref().ok_or_else(|| anyhow!("No NCN address"))
    }

    #[allow(clippy::large_stack_frames)]
    pub async fn handle(&self, action: ProgramCommand) -> Result<()> {
        match action {
            // Keeper
            ProgramCommand::Keeper {
                loop_timeout_ms,
                error_timeout_ms,
                test_vote,
            } => startup_keeper(self, loop_timeout_ms, error_timeout_ms, test_vote).await,

            // Admin
            ProgramCommand::AdminCreateConfig {
                epochs_before_stall,
                valid_slots_after_consensus,
                epochs_after_consensus_before_close,
                dao_fee_bps,
                block_engine_fee_bps,
                default_ncn_fee_bps,
                fee_wallet,
                tie_breaker_admin,
            } => {
                let fee_wallet =
                    fee_wallet.map(|s| Pubkey::from_str(&s).expect("error parsing fee wallet"));
                let tie_breaker = tie_breaker_admin
                    .map(|s| Pubkey::from_str(&s).expect("error parsing tie breaker admin"));
                admin_create_config(
                    self,
                    epochs_before_stall,
                    valid_slots_after_consensus,
                    epochs_after_consensus_before_close,
                    dao_fee_bps,
                    block_engine_fee_bps,
                    default_ncn_fee_bps,
                    fee_wallet,
                    tie_breaker,
                )
                .await
            }
            ProgramCommand::AdminRegisterStMint {
                vault,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                let switchboard = switchboard_feed
                    .map(|s| Pubkey::from_str(&s).expect("error parsing switchboard feed"));
                let ncn_fee_group =
                    NcnFeeGroup::try_from(ncn_fee_group).expect("error parsing fee group");
                admin_register_st_mint(
                    self,
                    &vault,
                    ncn_fee_group,
                    reward_multiplier_bps,
                    switchboard,
                    no_feed_weight,
                )
                .await
            }
            ProgramCommand::AdminSetWeight { vault, weight } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                admin_set_weight(self, &vault, self.epoch, weight).await
            }
            ProgramCommand::AdminSetTieBreaker { meta_merkle_root } => {
                todo!(
                    "Create and implement admin set tie breaker: {}",
                    meta_merkle_root
                );
                // let merkle_root = hex::decode(meta_merkle_root).expect("error parsing merkle root");
                // let mut root = [0u8; 32];
                // root.copy_from_slice(&merkle_root);
                // admin_set_tie_breaker(self, root).await
            }
            ProgramCommand::AdminSetParameters {
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
                starting_valid_epoch,
            } => {
                admin_set_parameters(
                    self,
                    epochs_before_stall,
                    epochs_after_consensus_before_close,
                    valid_slots_after_consensus,
                    starting_valid_epoch,
                )
                .await?;
                let config = get_tip_router_config(self).await?;
                info!("\n\n--- Parameters Set ---\nepochs_before_stall: {}\nepochs_after_consensus_before_close: {}\nvalid_slots_after_consensus: {}\nstarting_valid_epoch: {}\n",
                    config.epochs_before_stall(),
                    config.epochs_after_consensus_before_close(),
                    config.valid_slots_after_consensus(),
                    config.starting_valid_epoch()
                );

                Ok(())
            }
            ProgramCommand::AdminSetConfigFees {
                new_block_engine_fee_bps,
                base_fee_group,
                new_base_fee_wallet,
                new_base_fee_bps,
                ncn_fee_group,
                new_ncn_fee_bps,
            } => {
                admin_set_config_fees(
                    self,
                    new_block_engine_fee_bps,
                    base_fee_group,
                    new_base_fee_wallet,
                    new_base_fee_bps,
                    ncn_fee_group,
                    new_ncn_fee_bps,
                )
                .await
            }
            ProgramCommand::AdminSetNewAdmin {
                new_admin,
                set_fee_admin,
                set_tie_breaker_admin,
            } => {
                let new_admin = Pubkey::from_str(&new_admin).expect("error parsing new admin");
                admin_set_new_admin(self, &new_admin, set_fee_admin, set_tie_breaker_admin).await
            }
            ProgramCommand::AdminFundAccountPayer { amount_in_sol } => {
                admin_fund_account_payer(self, amount_in_sol).await
            }

            // Instructions
            ProgramCommand::CreateVaultRegistry {} => create_vault_registry(self).await,

            ProgramCommand::RegisterVault { vault } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                register_vault(self, &vault).await
            }

            ProgramCommand::CreateEpochState {} => create_epoch_state(self, self.epoch).await,

            ProgramCommand::CreateWeightTable {} => create_weight_table(self, self.epoch).await,
            ProgramCommand::CrankSwitchboard { switchboard_feed } => {
                let switchboard_feed =
                    Pubkey::from_str(&switchboard_feed).expect("error parsing switchboard feed");
                crank_switchboard(self, &switchboard_feed).await
            }
            ProgramCommand::SetWeight { vault } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                set_weight(self, &vault, self.epoch).await
            }

            ProgramCommand::CreateEpochSnapshot {} => create_epoch_snapshot(self, self.epoch).await,
            ProgramCommand::CreateOperatorSnapshot { operator } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                create_operator_snapshot(self, &operator, self.epoch).await
            }
            ProgramCommand::SnapshotVaultOperatorDelegation { vault, operator } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                snapshot_vault_operator_delegation(self, &vault, &operator, self.epoch).await
            }

            ProgramCommand::CreateBallotBox {} => create_ballot_box(self, self.epoch).await,
            ProgramCommand::OperatorCastVote {
                operator,
                meta_merkle_root,
            } => {
                todo!(
                    "Create and implement admin cast vote: {} {}",
                    operator,
                    meta_merkle_root
                );
                // let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                // let merkle_root = hex::decode(meta_merkle_root).expect("error parsing merkle root");
                // let mut root = [0u8; 32];
                // root.copy_from_slice(&merkle_root);
                // admin_cast_vote(self, &operator, root).await
            }

            ProgramCommand::CreateBaseRewardRouter {} => {
                create_base_reward_router(self, self.epoch).await
            }

            ProgramCommand::CreateNcnRewardRouter {
                operator,
                ncn_fee_group,
            } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let ncn_fee_group =
                    NcnFeeGroup::try_from(ncn_fee_group).expect("error parsing fee group");
                create_ncn_reward_router(self, ncn_fee_group, &operator, self.epoch).await
            }

            ProgramCommand::RouteBaseRewards {} => route_base_rewards(self, self.epoch).await,

            ProgramCommand::RouteNcnRewards {
                operator,
                ncn_fee_group,
            } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let ncn_fee_group =
                    NcnFeeGroup::try_from(ncn_fee_group).expect("error parsing fee group");
                route_ncn_rewards(self, &operator, ncn_fee_group, self.epoch).await
            }

            ProgramCommand::DistributeBaseNcnRewards {
                operator,
                ncn_fee_group,
            } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let ncn_fee_group =
                    NcnFeeGroup::try_from(ncn_fee_group).expect("error parsing fee group");
                distribute_base_ncn_rewards(self, &operator, ncn_fee_group, self.epoch).await
            }

            // Getters
            ProgramCommand::GetNcn {} => {
                let ncn = get_ncn(self).await?;
                info!("NCN: {:?}", ncn);
                Ok(())
            }
            ProgramCommand::GetNcnOperatorState { operator } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let ncn_operator_state = get_ncn_operator_state(self, &operator).await?;
                info!("NCN Operator State: {:?}", ncn_operator_state);
                Ok(())
            }
            ProgramCommand::GetVaultNcnTicket { vault } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                let ncn_ticket = get_vault_ncn_ticket(self, &vault).await?;
                info!("Vault NCN Ticket: {:?}", ncn_ticket);
                Ok(())
            }
            ProgramCommand::GetNcnVaultTicket { vault } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                let ncn_ticket = get_ncn_vault_ticket(self, &vault).await?;
                info!("NCN Vault Ticket: {:?}", ncn_ticket);
                Ok(())
            }
            ProgramCommand::GetVaultOperatorDelegation { vault, operator } => {
                let vault = Pubkey::from_str(&vault).expect("error parsing vault");
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");

                let vault_operator_delegation =
                    get_vault_operator_delegation(self, &vault, &operator).await?;

                info!("Vault Operator Delegation: {:?}", vault_operator_delegation);
                Ok(())
            }
            ProgramCommand::GetAllOperatorsInNcn {} => {
                let operators = get_all_operators_in_ncn(self).await?;

                info!("Operators: {:?}", operators);
                Ok(())
            }
            ProgramCommand::GetAllVaultsInNcn {} => {
                let vaults = get_all_vaults_in_ncn(self).await?;
                info!("Vaults: {:?}", vaults);
                Ok(())
            }
            ProgramCommand::GetAllTickets {} => {
                let all_tickets = get_all_tickets(self).await?;

                for tickets in all_tickets.iter() {
                    info!("Tickets: {}", tickets);
                }

                Ok(())
            }
            ProgramCommand::GetTipRouterConfig {} => {
                let config = get_tip_router_config(self).await?;
                info!("{}", config);
                Ok(())
            }
            ProgramCommand::GetVaultRegistry {} => {
                let vault_registry = get_vault_registry(self).await?;
                info!("{}", vault_registry);
                Ok(())
            }
            ProgramCommand::GetWeightTable {} => {
                let weight_table = get_weight_table(self, self.epoch).await?;
                info!("{}", weight_table);
                Ok(())
            }
            ProgramCommand::GetEpochState {} => {
                let is_epoch_complete = get_is_epoch_completed(self, self.epoch).await?;

                if is_epoch_complete {
                    info!("\n\nEpoch {} is complete", self.epoch);
                    return Ok(());
                }

                let epoch_state = get_epoch_state(self, self.epoch).await?;
                let current_slot = get_current_slot(self).await?;
                let current_state = {
                    let (valid_slots_after_consensus, epochs_after_consensus_before_close) = {
                        let config = get_tip_router_config(self).await?;
                        (
                            config.valid_slots_after_consensus(),
                            config.epochs_after_consensus_before_close(),
                        )
                    };
                    let epoch_schedule = self.rpc_client().get_epoch_schedule().await?;

                    if epoch_state.set_weight_progress().tally() > 0 {
                        let weight_table = get_weight_table(self, self.epoch).await?;
                        epoch_state.current_state_patched(
                            &epoch_schedule,
                            valid_slots_after_consensus,
                            epochs_after_consensus_before_close,
                            weight_table.st_mint_count() as u64,
                            current_slot,
                        )
                    } else {
                        epoch_state.current_state(
                            &epoch_schedule,
                            valid_slots_after_consensus,
                            epochs_after_consensus_before_close,
                            current_slot,
                        )
                    }
                };

                info!("{}\nCurrent State: {:?}\n", epoch_state, current_state);

                Ok(())
            }
            ProgramCommand::GetEpochSnapshot {} => {
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                info!("{}", epoch_snapshot);
                Ok(())
            }
            ProgramCommand::GetOperatorSnapshot { operator } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let operator_snapshot = get_operator_snapshot(self, &operator, self.epoch).await?;
                info!("{}", operator_snapshot);
                Ok(())
            }
            ProgramCommand::GetBallotBox {} => {
                let ballot_box = get_ballot_box(self, self.epoch).await?;
                info!("{}", ballot_box);
                Ok(())
            }
            ProgramCommand::GetBaseRewardReceiverAddress {} => {
                let (base_reward_receiver_address, _, _) = BaseRewardReceiver::find_program_address(
                    &self.tip_router_program_id,
                    self.ncn()?,
                    self.epoch,
                );
                info!("Base Reward Receiver: {}", base_reward_receiver_address);
                Ok(())
            }
            ProgramCommand::GetBaseRewardRouter {} => {
                let total_rewards_to_be_distributed =
                    get_total_rewards_to_be_distributed(self, self.epoch).await?;
                let base_reward_router = get_base_reward_router(self, self.epoch).await?;
                let (base_reward_receiver_address, base_reward_receiver_account) =
                    get_base_reward_receiver(self, self.epoch).await?;
                let rent = self
                    .rpc_client
                    .get_minimum_balance_for_rent_exemption(0)
                    .await?;
                info!(
                    "{}\nTotal Rewards To Distribute: {}\nReceiver {}: {}\n",
                    base_reward_router,
                    total_rewards_to_be_distributed,
                    base_reward_receiver_address,
                    base_reward_receiver_account.lamports - rent
                );
                Ok(())
            }
            ProgramCommand::GetNcnRewardRouter {
                operator,
                ncn_fee_group,
            } => {
                let operator = Pubkey::from_str(&operator).expect("error parsing operator");
                let ncn_fee_group =
                    NcnFeeGroup::try_from(ncn_fee_group).expect("error parsing fee group");
                let ncn_reward_router =
                    get_ncn_reward_router(self, ncn_fee_group, &operator, self.epoch).await?;
                let (ncn_reward_receiver_address, ncn_reward_receiver_account) =
                    get_ncn_reward_receiver(self, ncn_fee_group, &operator, self.epoch).await?;
                let rent = self
                    .rpc_client
                    .get_minimum_balance_for_rent_exemption(0)
                    .await?;
                info!(
                    "{}\nReceiver {}: {}\n",
                    ncn_reward_router,
                    ncn_reward_receiver_address,
                    ncn_reward_receiver_account.lamports - rent
                );
                Ok(())
            }
            ProgramCommand::GetAllNcnRewardRouters {} => {
                let all_operators = get_all_operators_in_ncn(self).await?;
                let rent = self
                    .rpc_client
                    .get_minimum_balance_for_rent_exemption(0)
                    .await?;
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                let fees = epoch_snapshot.fees();

                let mut valid_ncn_groups: Vec<NcnFeeGroup> = Vec::new();
                for group in NcnFeeGroup::all_groups() {
                    if fees.ncn_fee_bps(group)? > 0 {
                        valid_ncn_groups.push(group);
                    }
                }

                for operator in all_operators.iter() {
                    for group in valid_ncn_groups.iter() {
                        let ncn_reward_router =
                            get_ncn_reward_router(self, *group, operator, self.epoch).await?;
                        let (ncn_reward_receiver_address, ncn_reward_receiver_account) =
                            get_ncn_reward_receiver(self, *group, operator, self.epoch).await?;

                        info!(
                            "{}\nReceiver {}: {}\n",
                            ncn_reward_router,
                            ncn_reward_receiver_address,
                            ncn_reward_receiver_account.lamports - rent,
                        );
                    }
                }

                Ok(())
            }
            ProgramCommand::GetAccountPayer {} => {
                let account_payer = get_account_payer(self).await?;
                let (account_payer_address, _, _) =
                    AccountPayer::find_program_address(&self.tip_router_program_id, self.ncn()?);
                info!(
                    "\n\n--- Account Payer ---\n{}\nBalance: {}\n",
                    account_payer_address,
                    lamports_to_sol(account_payer.lamports)
                );
                Ok(())
            }
            ProgramCommand::GetTotalEpochRentCost {} => {
                let total_epoch_rent_cost = get_total_epoch_rent_cost(self).await?;
                info!(
                    "\n\n--- Total Epoch Rent Cost ---\nCost: {}\n",
                    lamports_to_sol(total_epoch_rent_cost)
                );
                Ok(())
            }
            ProgramCommand::GetStakePool {} => {
                let stake_pool = get_stake_pool(self).await?;
                info!("Stake Pool: {:?}", stake_pool);
                Ok(())
            }

            ProgramCommand::GetOperatorStakes {} => {
                // Get epoch snapshot for total stake
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;

                let operators = get_all_operators_in_ncn(self).await?;
                // For each fully activated operator, get their operator snapshot
                let mut operator_stakes = Vec::new();
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        operator_stakes
                            .push((operator, operator_snapshot.stake_weights().stake_weight()));
                    }
                }

                // Sort operator stakes by stake weight descending
                operator_stakes.sort_by(|(_, a), (_, b)| b.cmp(a));

                for (operator, stake_weight) in operator_stakes.iter() {
                    println!(
                        "Operator: {}, Stake Weight: {}.{:02}%",
                        operator,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() / 100,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() % 100
                    );
                }

                Ok(())
            }

            ProgramCommand::GetVaultStakes {} => {
                let operators = get_all_operators_in_ncn(self).await?;
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                let mut vault_stakes = HashMap::new();
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        for vault_operator_stake_weight in
                            operator_snapshot.vault_operator_stake_weight()
                        {
                            let vault = vault_operator_stake_weight.vault();

                            if *vault == Pubkey::default() {
                                continue;
                            }

                            let stake_weight =
                                vault_operator_stake_weight.stake_weights().stake_weight();

                            vault_stakes
                                .entry(*vault)
                                .and_modify(|w| *w += stake_weight)
                                .or_insert(stake_weight);
                        }
                    }
                }

                let mut vault_stakes = vault_stakes.into_iter().collect::<Vec<_>>();
                vault_stakes.sort_by(|(_, a), (_, b)| b.cmp(a));

                for (vault, stake_weight) in vault_stakes.iter() {
                    println!(
                        "Vault: {}, Stake Weight: {}.{:02}%",
                        vault,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() / 100,
                        stake_weight * 10000 / epoch_snapshot.stake_weights().stake_weight() % 100
                    );
                }

                Ok(())
            }

            ProgramCommand::GetVaultOperatorStakes {} => {
                let operators = get_all_operators_in_ncn(self).await?;
                let epoch_snapshot = get_epoch_snapshot(self, self.epoch).await?;
                let mut vault_operator_stakes: HashMap<Pubkey, HashMap<Pubkey, u128>> =
                    HashMap::new();

                // Collect stakes for each vault-operator pair
                for operator in operators.iter() {
                    let operator_snapshot = get_operator_snapshot(self, operator, self.epoch).await;
                    if let Ok(operator_snapshot) = operator_snapshot {
                        for vault_operator_stake_weight in
                            operator_snapshot.vault_operator_stake_weight()
                        {
                            let vault = vault_operator_stake_weight.vault();
                            if *vault == Pubkey::default() {
                                continue;
                            }
                            let stake_weight =
                                vault_operator_stake_weight.stake_weights().stake_weight();

                            vault_operator_stakes
                                .entry(*vault)
                                .or_default()
                                .insert(*operator, stake_weight);
                        }
                    }
                }

                // Calculate total stake weight for percentage calculations
                let total_stake_weight = epoch_snapshot.stake_weights().stake_weight();

                // Sort vaults by total stake
                let mut vaults: Vec<_> = vault_operator_stakes.iter().collect();
                vaults.sort_by(|(_, a_ops), (_, b_ops)| {
                    let a_total: u128 = a_ops.values().sum();
                    let b_total: u128 = b_ops.values().sum();
                    b_total.cmp(&a_total)
                });

                for (vault, operator_stakes) in vaults {
                    let vault_total: u128 = operator_stakes.values().sum();
                    if vault_total == 0 {
                        continue;
                    }
                    println!(
                        "Vault: {}, % of Total Stake: {}.{:02}%",
                        vault,
                        vault_total * 10000 / total_stake_weight / 100,
                        vault_total * 10000 / total_stake_weight % 100
                    );

                    let mut operators: Vec<_> = operator_stakes.iter().collect();
                    operators.sort_by(|(_, a), (_, b)| b.cmp(a));

                    for (operator, stake) in operators {
                        if *stake == 0 {
                            continue;
                        }
                        println!(
                            "  Operator: {}, Stake: {}.{:02}%",
                            operator,
                            stake * 10000 / vault_total / 100,
                            stake * 10000 / vault_total % 100
                        );
                    }
                    println!();
                }

                Ok(())
            }

            // Testers
            ProgramCommand::Test {} => {
                info!("Test!");
                Ok(())
            }
            ProgramCommand::CreateTestNcn {} => create_test_ncn(self).await,
            ProgramCommand::CreateAndAddTestOperator { operator_fee_bps } => {
                create_and_add_test_operator(self, operator_fee_bps).await
            }
            ProgramCommand::CreateAndAddTestVault {
                deposit_fee_bps,
                withdrawal_fee_bps,
                reward_fee_bps,
            } => {
                create_and_add_test_vault(self, deposit_fee_bps, withdrawal_fee_bps, reward_fee_bps)
                    .await
            }
        }
    }
}

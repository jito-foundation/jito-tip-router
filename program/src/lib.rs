mod admin_initialize_config;
mod admin_register_st_mint;
mod admin_set_config_fees;
mod admin_set_new_admin;
mod admin_set_parameters;
mod admin_set_st_mint;
mod admin_set_tie_breaker;
mod admin_set_weight;
mod cast_vote;
mod claim_with_payer;
mod close_epoch_account;
mod distribute_base_ncn_reward_route;
mod distribute_base_rewards;
mod distribute_ncn_operator_rewards;
mod distribute_ncn_vault_rewards;
mod initialize_ballot_box;
mod initialize_base_reward_router;
mod initialize_epoch_snapshot;
mod initialize_epoch_state;
mod initialize_ncn_reward_router;
mod initialize_operator_snapshot;
mod initialize_vault_registry;
mod initialize_weight_table;
mod realloc_ballot_box;
mod realloc_base_reward_router;
mod realloc_epoch_state;
mod realloc_operator_snapshot;
mod realloc_vault_registry;
mod realloc_weight_table;
mod register_vault;
mod route_base_rewards;
mod route_ncn_rewards;
mod set_merkle_root;
mod snapshot_vault_operator_delegation;
mod switchboard_set_weight;

use admin_set_new_admin::process_admin_set_new_admin;
use borsh::BorshDeserialize;
use const_str_to_pubkey::str_to_pubkey;
use initialize_epoch_state::process_initialize_epoch_state;
use jito_tip_router_core::instruction::TipRouterInstruction;
use realloc_epoch_state::process_realloc_epoch_state;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    admin_initialize_config::process_admin_initialize_config,
    admin_register_st_mint::process_admin_register_st_mint,
    admin_set_config_fees::process_admin_set_config_fees,
    admin_set_parameters::process_admin_set_parameters,
    admin_set_st_mint::process_admin_set_st_mint,
    admin_set_tie_breaker::process_admin_set_tie_breaker,
    admin_set_weight::process_admin_set_weight, cast_vote::process_cast_vote,
    claim_with_payer::process_claim_with_payer, close_epoch_account::process_close_epoch_account,
    distribute_base_ncn_reward_route::process_distribute_base_ncn_reward_route,
    distribute_base_rewards::process_distribute_base_rewards,
    distribute_ncn_operator_rewards::process_distribute_ncn_operator_rewards,
    distribute_ncn_vault_rewards::process_distribute_ncn_vault_rewards,
    initialize_ballot_box::process_initialize_ballot_box,
    initialize_base_reward_router::process_initialize_base_reward_router,
    initialize_epoch_snapshot::process_initialize_epoch_snapshot,
    initialize_ncn_reward_router::process_initialize_ncn_reward_router,
    initialize_operator_snapshot::process_initialize_operator_snapshot,
    initialize_vault_registry::process_initialize_vault_registry,
    initialize_weight_table::process_initialize_weight_table,
    realloc_ballot_box::process_realloc_ballot_box,
    realloc_base_reward_router::process_realloc_base_reward_router,
    realloc_operator_snapshot::process_realloc_operator_snapshot,
    realloc_vault_registry::process_realloc_vault_registry,
    realloc_weight_table::process_realloc_weight_table, register_vault::process_register_vault,
    route_base_rewards::process_route_base_rewards, route_ncn_rewards::process_route_ncn_rewards,
    set_merkle_root::process_set_merkle_root,
    snapshot_vault_operator_delegation::process_snapshot_vault_operator_delegation,
    switchboard_set_weight::process_switchboard_set_weight,
};

declare_id!(str_to_pubkey(env!("TIP_ROUTER_PROGRAM_ID")));

#[cfg(not(feature = "no-entrypoint"))]
security_txt! {
    // Required fields
    name: "Jito's MEV Tip Distribution NCN Program",
    project_url: "https://jito.network/",
    contacts: "email:team@jito.network",
    policy: "https://github.com/jito-foundation/jito-tip-router",
    // Optional Fields
    preferred_languages: "en",
    source_code: "https://github.com/jito-foundation/jito-tip-router"
}

#[cfg(not(feature = "no-entrypoint"))]
solana_program::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if *program_id != id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let instruction = TipRouterInstruction::try_from_slice(instruction_data)?;

    match instruction {
        // ---------------------------------------------------- //
        //                         GLOBAL                       //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeConfig {
            block_engine_fee_bps,
            dao_fee_bps,
            default_ncn_fee_bps,
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
        } => {
            msg!("Instruction: InitializeConfig");
            process_admin_initialize_config(
                program_id,
                accounts,
                block_engine_fee_bps,
                dao_fee_bps,
                default_ncn_fee_bps,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
            )
        }
        TipRouterInstruction::InitializeVaultRegistry => {
            msg!("Instruction: InitializeVaultRegistry");
            process_initialize_vault_registry(program_id, accounts)
        }
        TipRouterInstruction::ReallocVaultRegistry => {
            msg!("Instruction: ReallocVaultRegistry");
            process_realloc_vault_registry(program_id, accounts)
        }
        TipRouterInstruction::RegisterVault => {
            msg!("Instruction: RegisterVault");
            process_register_vault(program_id, accounts)
        }

        // ---------------------------------------------------- //
        //                       SNAPSHOT                       //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeEpochState { epoch } => {
            msg!("Instruction: InitializeEpochState");
            process_initialize_epoch_state(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocEpochState { epoch } => {
            msg!("Instruction: ReallocEpochState");
            process_realloc_epoch_state(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeWeightTable { epoch } => {
            msg!("Instruction: InitializeWeightTable");
            process_initialize_weight_table(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocWeightTable { epoch } => {
            msg!("Instruction: ReallocWeightTable");
            process_realloc_weight_table(program_id, accounts, epoch)
        }
        TipRouterInstruction::SwitchboardSetWeight { epoch, st_mint } => {
            msg!("Instruction: SwitchboardSetWeight");
            process_switchboard_set_weight(program_id, accounts, &st_mint, epoch)
        }
        TipRouterInstruction::InitializeEpochSnapshot { epoch } => {
            msg!("Instruction: InitializeEpochSnapshot");
            process_initialize_epoch_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeOperatorSnapshot { epoch } => {
            msg!("Instruction: InitializeOperatorSnapshot");
            process_initialize_operator_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocOperatorSnapshot { epoch } => {
            msg!("Instruction: ReallocOperatorSnapshot");
            process_realloc_operator_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::SnapshotVaultOperatorDelegation { epoch } => {
            msg!("Instruction: SnapshotVaultOperatorDelegation");
            process_snapshot_vault_operator_delegation(program_id, accounts, epoch)
        }

        // ---------------------------------------------------- //
        //                         VOTE                         //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeBallotBox { epoch } => {
            msg!("Instruction: InitializeBallotBox");
            process_initialize_ballot_box(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocBallotBox { epoch } => {
            msg!("Instruction: ReallocBallotBox");
            process_realloc_ballot_box(program_id, accounts, epoch)
        }
        TipRouterInstruction::CastVote {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: CastVote");
            process_cast_vote(program_id, accounts, &meta_merkle_root, epoch)
        }
        TipRouterInstruction::SetMerkleRoot {
            proof,
            merkle_root,
            max_total_claim,
            max_num_nodes,
            epoch,
        } => {
            msg!("Instruction: SetMerkleRoot");
            process_set_merkle_root(
                program_id,
                accounts,
                proof,
                merkle_root,
                max_total_claim,
                max_num_nodes,
                epoch,
            )
        }

        // ---------------------------------------------------- //
        //                ROUTE AND DISTRIBUTE                  //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeBaseRewardRouter { epoch } => {
            msg!("Instruction: InitializeBaseRewardRouter");
            process_initialize_base_reward_router(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocBaseRewardRouter { epoch } => {
            msg!("Instruction: ReallocBaseRewardRouter");
            process_realloc_base_reward_router(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeNcnRewardRouter {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: InitializeNcnRewardRouter");
            process_initialize_ncn_reward_router(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::RouteBaseRewards {
            max_iterations,
            epoch,
        } => {
            msg!("Instruction: RouteBaseRewards");
            process_route_base_rewards(program_id, accounts, max_iterations, epoch)
        }
        TipRouterInstruction::RouteNcnRewards {
            ncn_fee_group,
            max_iterations,
            epoch,
        } => {
            msg!("Instruction: RouteNcnRewards");
            process_route_ncn_rewards(program_id, accounts, ncn_fee_group, max_iterations, epoch)
        }
        TipRouterInstruction::DistributeBaseRewards {
            base_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeBaseRewards");
            process_distribute_base_rewards(program_id, accounts, base_fee_group, epoch)
        }
        TipRouterInstruction::DistributeBaseNcnRewardRoute {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeBaseNcnRewardRoute");
            process_distribute_base_ncn_reward_route(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::DistributeNcnOperatorRewards {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeNcnOperatorRewards");
            process_distribute_ncn_operator_rewards(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::DistributeNcnVaultRewards {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeNcnVaultRewards");
            process_distribute_ncn_vault_rewards(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::ClaimWithPayer {
            proof,
            amount,
            bump,
        } => {
            msg!("Instruction: ClaimWithPayer");
            process_claim_with_payer(program_id, accounts, proof, amount, bump)
        }
        TipRouterInstruction::CloseEpochAccount { epoch } => {
            msg!("Instruction: CloseEpochAccount");
            process_close_epoch_account(program_id, accounts, epoch)
        }

        // ---------------------------------------------------- //
        //                        ADMIN                         //
        // ---------------------------------------------------- //
        TipRouterInstruction::AdminSetParameters {
            starting_valid_epoch,
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
        } => {
            msg!("Instruction: AdminSetParameters");
            process_admin_set_parameters(
                program_id,
                accounts,
                starting_valid_epoch,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
            )
        }
        TipRouterInstruction::AdminSetConfigFees {
            new_block_engine_fee_bps,
            base_fee_group,
            new_base_fee_wallet,
            new_base_fee_bps,
            ncn_fee_group,
            new_ncn_fee_bps,
        } => {
            msg!("Instruction: AdminSetConfigFees");
            process_admin_set_config_fees(
                program_id,
                accounts,
                new_block_engine_fee_bps,
                base_fee_group,
                new_base_fee_wallet,
                new_base_fee_bps,
                ncn_fee_group,
                new_ncn_fee_bps,
            )
        }
        TipRouterInstruction::AdminSetNewAdmin { role } => {
            msg!("Instruction: AdminSetNewAdmin");
            process_admin_set_new_admin(program_id, accounts, role)
        }
        TipRouterInstruction::AdminSetTieBreaker {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: AdminSetTieBreaker");
            process_admin_set_tie_breaker(program_id, accounts, &meta_merkle_root, epoch)
        }
        TipRouterInstruction::AdminSetWeight {
            st_mint,
            weight,
            epoch,
        } => {
            msg!("Instruction: AdminSetWeight");
            process_admin_set_weight(program_id, accounts, &st_mint, epoch, weight)
        }
        TipRouterInstruction::AdminRegisterStMint {
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        } => {
            msg!("Instruction: AdminRegisterStMint");
            process_admin_register_st_mint(
                program_id,
                accounts,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            )
        }
        TipRouterInstruction::AdminSetStMint {
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        } => {
            msg!("Instruction: AdminSetStMint");
            process_admin_set_st_mint(
                program_id,
                accounts,
                &st_mint,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            )
        }
    }
}

mod admin_update_weight_table;
mod cast_vote;
mod distribute_base_ncn_reward_route;
mod distribute_base_rewards;
mod distribute_ncn_operator_rewards;
mod distribute_ncn_vault_rewards;
mod initialize_ballot_box;
mod initialize_base_reward_router;
mod initialize_epoch_snapshot;
mod initialize_ncn_config;
mod initialize_ncn_reward_router;
mod initialize_operator_snapshot;
mod initialize_tracked_mints;
mod initialize_weight_table;
mod register_mint;
mod route_base_rewards;
mod route_ncn_rewards;
mod set_config_fees;
mod set_merkle_root;
mod set_new_admin;
mod set_tie_breaker;
mod set_tracked_mint_ncn_fee_group;
mod snapshot_vault_operator_delegation;

use borsh::BorshDeserialize;
use const_str_to_pubkey::str_to_pubkey;
use jito_tip_router_core::instruction::TipRouterInstruction;
use set_new_admin::process_set_new_admin;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    admin_update_weight_table::process_admin_update_weight_table, cast_vote::process_cast_vote,
    distribute_base_ncn_reward_route::process_distribute_base_ncn_reward_route,
    distribute_base_rewards::process_distribute_base_rewards,
    distribute_ncn_operator_rewards::process_distribute_ncn_operator_rewards,
    distribute_ncn_vault_rewards::process_distribute_ncn_vault_rewards,
    initialize_ballot_box::process_initialize_ballot_box,
    initialize_base_reward_router::process_initialize_base_reward_router,
    initialize_epoch_snapshot::process_initialize_epoch_snapshot,
    initialize_ncn_config::process_initialize_ncn_config,
    initialize_ncn_reward_router::process_initialize_ncn_reward_router,
    initialize_operator_snapshot::process_initialize_operator_snapshot,
    initialize_tracked_mints::process_initialize_tracked_mints,
    initialize_weight_table::process_initialize_weight_table, register_mint::process_register_mint,
    route_base_rewards::process_route_base_rewards, route_ncn_rewards::process_route_ncn_rewards,
    set_config_fees::process_set_config_fees, set_merkle_root::process_set_merkle_root,
    set_tie_breaker::process_set_tie_breaker,
    set_tracked_mint_ncn_fee_group::process_set_tracked_mint_ncn_fee_group,
    snapshot_vault_operator_delegation::process_snapshot_vault_operator_delegation,
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
        // ------------------------------------------
        // Initialization
        // ------------------------------------------
        TipRouterInstruction::InitializeNCNConfig {
            block_engine_fee_bps,
            dao_fee_bps,
            default_ncn_fee_bps,
        } => {
            msg!("Instruction: InitializeConfig");
            process_initialize_ncn_config(
                program_id,
                accounts,
                block_engine_fee_bps,
                dao_fee_bps,
                default_ncn_fee_bps,
            )
        }
        TipRouterInstruction::InitializeTrackedMints => {
            msg!("Instruction: InitializeTrackedMints");
            process_initialize_tracked_mints(program_id, accounts)
        }
        TipRouterInstruction::InitializeWeightTable {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeWeightTable");
            process_initialize_weight_table(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::InitializeEpochSnapshot {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeEpochSnapshot");
            process_initialize_epoch_snapshot(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::InitializeOperatorSnapshot {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeOperatorSnapshot");
            process_initialize_operator_snapshot(program_id, accounts, first_slot_of_ncn_epoch)
        }

        TipRouterInstruction::InitializeBaseRewardRouter {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeBaseRewardRouter");
            process_initialize_base_reward_router(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::InitializeNcnRewardRouter {
            ncn_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeNcnRewardRouter");
            process_initialize_ncn_reward_router(
                program_id,
                accounts,
                ncn_fee_group,
                first_slot_of_ncn_epoch,
            )
        }
        // ------------------------------------------
        // Cranks
        // ------------------------------------------
        TipRouterInstruction::SnapshotVaultOperatorDelegation {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: SnapshotVaultOperatorDelegation");
            process_snapshot_vault_operator_delegation(
                program_id,
                accounts,
                first_slot_of_ncn_epoch,
            )
        }
        TipRouterInstruction::RouteBaseRewards {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: RouteBaseRewards");
            process_route_base_rewards(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::RouteNcnRewards {
            ncn_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: RouteNcnRewards");
            process_route_ncn_rewards(program_id, accounts, ncn_fee_group, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::DistributeBaseRewards {
            base_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: DistributeBaseRewards");
            process_distribute_base_rewards(
                program_id,
                accounts,
                base_fee_group,
                first_slot_of_ncn_epoch,
            )
        }
        TipRouterInstruction::DistributeBaseNcnRewardRoute {
            ncn_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: DistributeBaseNcnRewardRoute");
            process_distribute_base_ncn_reward_route(
                program_id,
                accounts,
                ncn_fee_group,
                first_slot_of_ncn_epoch,
            )
        }
        TipRouterInstruction::DistributeNcnOperatorRewards {
            ncn_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: DistributeNcnOperatorRewards");
            process_distribute_ncn_operator_rewards(
                program_id,
                accounts,
                ncn_fee_group,
                first_slot_of_ncn_epoch,
            )
        }
        TipRouterInstruction::DistributeNcnVaultRewards {
            ncn_fee_group,
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: DistributeNcnVaultRewards");
            process_distribute_ncn_vault_rewards(
                program_id,
                accounts,
                ncn_fee_group,
                first_slot_of_ncn_epoch,
            )
        }
        // ------------------------------------------
        // Update
        // ------------------------------------------
        TipRouterInstruction::AdminUpdateWeightTable { ncn_epoch, weight } => {
            msg!("Instruction: UpdateWeightTable");
            process_admin_update_weight_table(program_id, accounts, ncn_epoch, weight)
        }
        TipRouterInstruction::SetConfigFees {
            new_block_engine_fee_bps,
            base_fee_group,
            new_base_fee_wallet,
            new_base_fee_bps,
            ncn_fee_group,
            new_ncn_fee_bps,
        } => {
            msg!("Instruction: SetConfigFees");
            process_set_config_fees(
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
        TipRouterInstruction::SetNewAdmin { role } => {
            msg!("Instruction: SetNewAdmin");
            process_set_new_admin(program_id, accounts, role)
        }
        TipRouterInstruction::RegisterMint => {
            msg!("Instruction: RegisterMint");
            process_register_mint(program_id, accounts)
        }
        TipRouterInstruction::SetTrackedMintNcnFeeGroup {
            vault_index,
            ncn_fee_group,
        } => {
            msg!("Instruction: SetTrackedMintNcnFeeGroup");
            process_set_tracked_mint_ncn_fee_group(program_id, accounts, vault_index, ncn_fee_group)
        }
        TipRouterInstruction::InitializeBallotBox { epoch } => {
            msg!("Instruction: InitializeBallotBox");
            process_initialize_ballot_box(program_id, accounts, epoch)
        }
        TipRouterInstruction::CastVote {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: CastVote");
            process_cast_vote(program_id, accounts, meta_merkle_root, epoch)
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
        TipRouterInstruction::SetTieBreaker {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: SetTieBreaker");
            process_set_tie_breaker(program_id, accounts, meta_merkle_root, epoch)
        }
    }
}

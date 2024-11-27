mod admin_update_weight_table;
mod initialize_epoch_reward_router;
mod initialize_epoch_snapshot;
mod initialize_ncn_config;
mod initialize_operator_epoch_reward_router;
mod initialize_operator_snapshot;
mod initialize_tracked_mints;
mod initialize_weight_table;
mod process_epoch_reward_buckets;
mod process_epoch_reward_pool;
mod process_operator_epoch_reward_pool;
mod register_mint;
mod set_config_fees;
mod set_new_admin;
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
    admin_update_weight_table::process_admin_update_weight_table,
    initialize_epoch_reward_router::process_initialize_epoch_reward_router,
    initialize_epoch_snapshot::process_initialize_epoch_snapshot,
    initialize_ncn_config::process_initialize_ncn_config,
    initialize_operator_epoch_reward_router::process_initialize_operator_epoch_reward_router,
    initialize_operator_snapshot::process_initialize_operator_snapshot,
    initialize_tracked_mints::process_initialize_tracked_mints,
    initialize_weight_table::process_initialize_weight_table,
    process_epoch_reward_buckets::process_process_epoch_reward_buckets,
    process_epoch_reward_pool::process_process_epoch_reward_pool,
    process_operator_epoch_reward_pool::process_process_operator_epoch_reward_pool,
    register_mint::process_register_mint, set_config_fees::process_set_config_fees,
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
            dao_fee_bps,
            ncn_fee_bps,
            block_engine_fee_bps,
        } => {
            msg!("Instruction: InitializeConfig");
            process_initialize_ncn_config(
                program_id,
                accounts,
                dao_fee_bps,
                ncn_fee_bps,
                block_engine_fee_bps,
            )
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
        TipRouterInstruction::SnapshotVaultOperatorDelegation {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeVaultOperatorDelegationSnapshot");
            process_snapshot_vault_operator_delegation(
                program_id,
                accounts,
                first_slot_of_ncn_epoch,
            )
        }
        TipRouterInstruction::InitializeEpochRewardRouter {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeEpochRewardRouter");
            process_initialize_epoch_reward_router(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::InitializeOperatorEpochRewardRouter {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeOperatorEpochRewardRouter");
            process_initialize_operator_epoch_reward_router(
                program_id,
                accounts,
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
            new_fee_wallet,
            new_block_engine_fee_bps,
            new_dao_fee_bps,
            new_ncn_fee_bps,
            new_ncn_fee_group,
        } => {
            msg!("Instruction: SetConfigFees");
            process_set_config_fees(
                program_id,
                accounts,
                new_fee_wallet,
                new_block_engine_fee_bps,
                new_dao_fee_bps,
                new_ncn_fee_bps,
                new_ncn_fee_group,
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
        TipRouterInstruction::InitializeTrackedMints => {
            msg!("Instruction: InitializeTrackedMints");
            process_initialize_tracked_mints(program_id, accounts)
        }
        TipRouterInstruction::ProcessEpochRewardPool {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: ProcessEpochRewardPool");
            process_process_epoch_reward_pool(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::ProcessEpochRewardBuckets {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: ProcessEpochRewardBuckets");
            process_process_epoch_reward_buckets(program_id, accounts, first_slot_of_ncn_epoch)
        }
        TipRouterInstruction::ProcessOperatorEpochRewardPool {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: ProcessOperatorEpochRewardPool");
            process_process_operator_epoch_reward_pool(
                program_id,
                accounts,
                first_slot_of_ncn_epoch,
            )
        }
    }
}

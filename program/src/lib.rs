mod admin_update_weight_table;
mod initialize_ncn_config;
mod initialize_weight_table;
mod register_mint;
mod set_config_fees;
mod set_new_admin;

use borsh::BorshDeserialize;
use const_str_to_pubkey::str_to_pubkey;
use jito_tip_router_core::instruction::WeightTableInstruction;
use set_new_admin::process_set_new_admin;
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint::ProgramResult, msg,
    program_error::ProgramError, pubkey::Pubkey,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;

use crate::{
    admin_update_weight_table::process_admin_update_weight_table,
    initialize_ncn_config::process_initialize_ncn_config,
    initialize_weight_table::process_initialize_weight_table, register_mint::process_register_mint,
    set_config_fees::process_set_config_fees,
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

    let instruction = WeightTableInstruction::try_from_slice(instruction_data)?;

    match instruction {
        // ------------------------------------------
        // Initialization
        // ------------------------------------------
        WeightTableInstruction::InitializeNCNConfig {
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
        WeightTableInstruction::InitializeWeightTable {
            first_slot_of_ncn_epoch,
        } => {
            msg!("Instruction: InitializeWeightTable");
            process_initialize_weight_table(program_id, accounts, first_slot_of_ncn_epoch)
        }
        // ------------------------------------------
        // Update
        // ------------------------------------------
        WeightTableInstruction::AdminUpdateWeightTable { ncn_epoch, weight } => {
            msg!("Instruction: UpdateWeightTable");
            process_admin_update_weight_table(program_id, accounts, ncn_epoch, weight)
        }
        WeightTableInstruction::SetConfigFees {
            new_dao_fee_bps,
            new_ncn_fee_bps,
            new_block_engine_fee_bps,
            new_fee_wallet,
        } => {
            msg!("Instruction: SetConfigFees");
            process_set_config_fees(
                program_id,
                accounts,
                new_dao_fee_bps,
                new_ncn_fee_bps,
                new_block_engine_fee_bps,
                new_fee_wallet,
            )
        }
        WeightTableInstruction::SetNewAdmin { role } => {
            msg!("Instruction: SetNewAdmin");
            process_set_new_admin(program_id, accounts, role)
        }
        WeightTableInstruction::RegisterMint => {
            msg!("Instruction: RegisterMint");
            process_register_mint(program_id, accounts)
        }
    }
}

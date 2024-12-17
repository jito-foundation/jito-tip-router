use anchor_lang::{
    prelude::Pubkey, solana_program::instruction::Instruction, InstructionData, ToAccountMetas,
};

use crate::jito_tip_distribution;

#[allow(clippy::too_many_arguments)]
pub fn initialize_ix(
    config: Pubkey,
    system_program: Pubkey,
    initializer: Pubkey,
    authority: Pubkey,
    expired_funds_account: Pubkey,
    num_epochs_valid: u64,
    max_validator_commission_bps: u16,
    bump: u8,
) -> Instruction {
    Instruction {
        program_id: jito_tip_distribution::ID,
        accounts: jito_tip_distribution::client::accounts::Initialize {
            config,
            system_program,
            initializer,
        }
        .to_account_metas(None),
        data: jito_tip_distribution::client::args::Initialize {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        }
        .data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_tip_distribution_account_ix(
    config: Pubkey,
    tip_distribution_account: Pubkey,
    system_program: Pubkey,
    validator_vote_account: Pubkey,
    signer: Pubkey,
    merkle_root_upload_authority: Pubkey,
    validator_commission_bps: u16,
    bump: u8,
) -> Instruction {
    Instruction {
        program_id: jito_tip_distribution::ID,
        accounts: jito_tip_distribution::client::accounts::InitializeTipDistributionAccount {
            config,
            tip_distribution_account,
            system_program,
            validator_vote_account,
            signer,
        }
        .to_account_metas(None),
        data: jito_tip_distribution::client::args::InitializeTipDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            bump,
        }
        .data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn claim_ix(
    config: Pubkey,
    tip_distribution_account: Pubkey,
    claim_status: Pubkey,
    claimant: Pubkey,
    payer: Pubkey,
    system_program: Pubkey,
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> Instruction {
    Instruction {
        program_id: jito_tip_distribution::ID,
        accounts: jito_tip_distribution::client::accounts::Claim {
            config,
            tip_distribution_account,
            claim_status,
            claimant,
            payer,
            system_program,
        }
        .to_account_metas(None),
        data: jito_tip_distribution::client::args::Claim {
            proof,
            amount,
            bump,
        }
        .data(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn upload_merkle_root_ix(
    config: Pubkey,
    merkle_root_upload_authority: Pubkey,
    tip_distribution_account: Pubkey,
    root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
) -> Instruction {
    Instruction {
        program_id: jito_tip_distribution::ID,
        accounts: jito_tip_distribution::client::accounts::UploadMerkleRoot {
            config,
            merkle_root_upload_authority,
            tip_distribution_account,
        }
        .to_account_metas(None),
        data: jito_tip_distribution::client::args::UploadMerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
        }
        .data(),
    }
}

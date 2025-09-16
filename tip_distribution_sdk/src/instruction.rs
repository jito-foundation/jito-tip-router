use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_sdk::instruction::AccountMeta;

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct Initialize {
    authority: Pubkey,
    expired_funds_account: Pubkey,
    num_epochs_valid: u64,
    max_validator_commission_bps: u16,
    bump: u8,
}

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
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(initializer, true),
        ],
        data: borsh::to_vec(&Initialize {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        })
        .expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct InitializeTipDistributionAccount {
    merkle_root_upload_authority: Pubkey,
    validator_commission_bps: u16,
    bump: u8,
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
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new(tip_distribution_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(validator_vote_account, false),
            AccountMeta::new_readonly(signer, true),
        ],
        data: borsh::to_vec(&InitializeTipDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            bump,
        })
        .expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct Claim {
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
}

#[allow(clippy::too_many_arguments)]
pub fn claim_ix(
    config: Pubkey,
    tip_distribution_account: Pubkey,
    merkle_root_upload_authority: Pubkey,
    claim_status: Pubkey,
    claimant: Pubkey,
    payer: Pubkey,
    system_program: Pubkey,
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new(tip_distribution_account, false),
            AccountMeta::new_readonly(merkle_root_upload_authority, false),
            AccountMeta::new(claim_status, false),
            AccountMeta::new_readonly(claimant, true),
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(system_program, false),
        ],
        data: borsh::to_vec(&Claim {
            proof,
            amount,
            bump,
        })
        .expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct UploadMerkleRoot {
    root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
}

pub fn upload_merkle_root_ix(
    config: Pubkey,
    merkle_root_upload_authority: Pubkey,
    tip_distribution_account: Pubkey,
    root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(merkle_root_upload_authority, true),
            AccountMeta::new(tip_distribution_account, false),
        ],
        data: borsh::to_vec(&UploadMerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
        })
        .expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct CloseClaimStatus {}

pub fn close_claim_status_ix(
    config: Pubkey,
    claim_status: Pubkey,
    claim_status_payer: Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new(claim_status, false),
            AccountMeta::new_readonly(claim_status_payer, true),
        ],
        data: borsh::to_vec(&CloseClaimStatus {}).expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct CloseTipDistributionAccount {
    _epoch: u64,
}

pub fn close_tip_distribution_account_ix(
    config: Pubkey,
    tip_distribution_account: Pubkey,
    expired_funds_account: Pubkey,
    validator_vote_account: Pubkey,
    signer: Pubkey,
    epoch: u64,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new(tip_distribution_account, false),
            AccountMeta::new(expired_funds_account, false),
            AccountMeta::new_readonly(validator_vote_account, false),
            AccountMeta::new_readonly(signer, true),
        ],
        data: borsh::to_vec(&CloseTipDistributionAccount { _epoch: epoch })
            .expect("Failed to serialize instruction data"),
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct MigrateTdaMerkleRootUploadAuthority {}

pub fn migrate_tda_merkle_root_upload_authority_ix(
    tip_distribution_account: Pubkey,
    merkle_root_upload_config: Pubkey,
) -> Instruction {
    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(tip_distribution_account, false),
            AccountMeta::new_readonly(merkle_root_upload_config, true),
        ],
        data: borsh::to_vec(&MigrateTdaMerkleRootUploadAuthority {})
            .expect("Failed to serialize instruction data"),
    }
}

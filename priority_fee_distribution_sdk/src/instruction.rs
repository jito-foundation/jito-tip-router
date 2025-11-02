use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::AccountMeta;
use solana_program::{instruction::Instruction, pubkey::Pubkey};

// Anchor discriminators from IDL
const INITIALIZE_DISCRIMINATOR: [u8; 8] = [175, 175, 109, 31, 13, 152, 155, 237];
const INITIALIZE_PRIORITY_FEE_DISTRIBUTION_ACCOUNT_DISCRIMINATOR: [u8; 8] =
    [49, 128, 247, 162, 140, 2, 193, 87];
const CLAIM_DISCRIMINATOR: [u8; 8] = [62, 198, 214, 193, 213, 159, 108, 210];
const UPLOAD_MERKLE_ROOT_DISCRIMINATOR: [u8; 8] = [70, 3, 110, 29, 199, 190, 205, 176];
const CLOSE_CLAIM_STATUS_DISCRIMINATOR: [u8; 8] = [163, 214, 191, 165, 245, 188, 17, 185];
const CLOSE_PRIORITY_FEE_DISTRIBUTION_ACCOUNT_DISCRIMINATOR: [u8; 8] =
    [127, 143, 71, 136, 78, 181, 210, 101];
const MIGRATE_TDA_MERKLE_ROOT_UPLOAD_AUTHORITY_DISCRIMINATOR: [u8; 8] =
    [13, 226, 163, 144, 56, 202, 214, 23];

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
    let mut data = INITIALIZE_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&Initialize {
            authority,
            expired_funds_account,
            num_epochs_valid,
            max_validator_commission_bps,
            bump,
        })
        .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new(initializer, true),
        ],
        data,
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct InitializeTipDistributionAccount {
    merkle_root_upload_authority: Pubkey,
    validator_commission_bps: u16,
    bump: u8,
}

#[allow(clippy::too_many_arguments)]
pub fn initialize_priority_fee_distribution_account_ix(
    config: Pubkey,
    priority_fee_distribution_account: Pubkey,
    system_program: Pubkey,
    validator_vote_account: Pubkey,
    signer: Pubkey,
    merkle_root_upload_authority: Pubkey,
    validator_commission_bps: u16,
    bump: u8,
) -> Instruction {
    let mut data = INITIALIZE_PRIORITY_FEE_DISTRIBUTION_ACCOUNT_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&InitializeTipDistributionAccount {
            merkle_root_upload_authority,
            validator_commission_bps,
            bump,
        })
        .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(priority_fee_distribution_account, false),
            AccountMeta::new_readonly(validator_vote_account, false),
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct Claim {
    _bump: u8,
    amount: u64,
    proof: Vec<[u8; 32]>,
}

#[allow(clippy::too_many_arguments)]
pub fn claim_ix(
    config: Pubkey,
    priority_fee_distribution_account: Pubkey,
    merkle_root_upload_authority: Pubkey,
    claim_status: Pubkey,
    claimant: Pubkey,
    payer: Pubkey,
    system_program: Pubkey,
    proof: Vec<[u8; 32]>,
    amount: u64,
    bump: u8,
) -> Instruction {
    let mut data = CLAIM_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&Claim {
            _bump: bump,
            amount,
            proof,
        })
        .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(priority_fee_distribution_account, false),
            AccountMeta::new_readonly(merkle_root_upload_authority, true),
            AccountMeta::new(claim_status, false),
            AccountMeta::new(claimant, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(system_program, false),
        ],
        data,
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
    priority_fee_distribution_account: Pubkey,
    root: [u8; 32],
    max_total_claim: u64,
    max_num_nodes: u64,
) -> Instruction {
    let mut data = UPLOAD_MERKLE_ROOT_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&UploadMerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
        })
        .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(priority_fee_distribution_account, false),
            AccountMeta::new(merkle_root_upload_authority, true),
        ],
        data,
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct CloseClaimStatus {}

pub fn close_claim_status_ix(
    _config: Pubkey,
    claim_status: Pubkey,
    claim_status_payer: Pubkey,
) -> Instruction {
    let mut data = CLOSE_CLAIM_STATUS_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&CloseClaimStatus {}).expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(claim_status, false),
            AccountMeta::new(claim_status_payer, false),
        ],
        data,
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct ClosePriorityFeeDistributionAccount {
    _epoch: u64,
}

pub fn close_priority_fee_distribution_account_ix(
    config: Pubkey,
    priority_fee_distribution_account: Pubkey,
    expired_funds_account: Pubkey,
    validator_vote_account: Pubkey,
    signer: Pubkey,
    epoch: u64,
) -> Instruction {
    let mut data = CLOSE_PRIORITY_FEE_DISTRIBUTION_ACCOUNT_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&ClosePriorityFeeDistributionAccount { _epoch: epoch })
            .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new_readonly(config, false),
            AccountMeta::new(expired_funds_account, false),
            AccountMeta::new(priority_fee_distribution_account, false),
            AccountMeta::new(validator_vote_account, false),
            AccountMeta::new(signer, true),
        ],
        data,
    }
}

#[derive(Debug, BorshDeserialize, BorshSerialize)]
struct MigrateTdaMerkleRootUploadAuthority {}

pub fn migrate_tda_merkle_root_upload_authority_ix(
    priority_fee_distribution_account: Pubkey,
    merkle_root_upload_config: Pubkey,
) -> Instruction {
    let mut data = MIGRATE_TDA_MERKLE_ROOT_UPLOAD_AUTHORITY_DISCRIMINATOR.to_vec();
    data.extend_from_slice(
        &borsh::to_vec(&MigrateTdaMerkleRootUploadAuthority {})
            .expect("Failed to serialize instruction data"),
    );

    Instruction {
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(priority_fee_distribution_account, false),
            AccountMeta::new_readonly(merkle_root_upload_config, false),
        ],
        data,
    }
}

use solana_program::instruction::Instruction;
use solana_pubkey::Pubkey;
use solana_sdk::instruction::AccountMeta;

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
        data: vec![], /*jito_tip_distribution::client::args::Initialize {
                          authority,
                          expired_funds_account,
                          num_epochs_valid,
                          max_validator_commission_bps,
                          bump,
                      }
                      .data(),*/
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
        program_id: crate::id(),
        accounts: vec![
            AccountMeta::new(config, false),
            AccountMeta::new(tip_distribution_account, false),
            AccountMeta::new_readonly(system_program, false),
            AccountMeta::new_readonly(validator_vote_account, false),
            AccountMeta::new_readonly(signer, true),
        ],
        data: vec![], /*jito_tip_distribution::client::args::InitializeTipDistributionAccount {
                          merkle_root_upload_authority,
                          validator_commission_bps,
                          bump,
                      }
                      .data(),*/
    }
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
        data: vec![], /*jito_tip_distribution::client::args::Claim {
                          proof,
                          amount,
                          bump,
                      }
                      .data(),*/
    }
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
        /*jito_tip_distribution::client::accounts::UploadMerkleRoot {
            config,
            merkle_root_upload_authority,
            tip_distribution_account,
        }
        .to_account_metas(None),*/
        data: vec![],
        /*jito_tip_distribution::client::args::UploadMerkleRoot {
            root,
            max_total_claim,
            max_num_nodes,
        }
        .data(),*/
    }
}

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
        data: vec![], //jito_tip_distribution::client::args::CloseClaimStatus {}.data(),
    }
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
        data: vec![], /*jito_tip_distribution::client::args::CloseTipDistributionAccount { _epoch: epoch }
                      .data(),*/
    }
}

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
        data: vec![], /*jito_tip_distribution::client::args::MigrateTdaMerkleRootUploadAuthority {}.data(),*/
    }
}

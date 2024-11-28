use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use solana_program::pubkey::Pubkey;

#[derive(Debug, BorshSerialize, BorshDeserialize)]
pub enum ConfigAdminRole {
    FeeAdmin,
    TieBreakerAdmin,
}

#[rustfmt::skip]
#[derive(Debug, BorshSerialize, BorshDeserialize, ShankInstruction)]
pub enum TipRouterInstruction {


    /// Initialize the global configuration for this NCN
    #[account(0, name = "restaking_config")]
    #[account(1, writable, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "fee_wallet")]
    #[account(5, name = "tie_breaker_admin")]
    #[account(6, name = "restaking_program")]
    #[account(7, name = "system_program")]
    InitializeNCNConfig {
        dao_fee_bps: u64,
        ncn_fee_bps: u64,
        block_engine_fee_bps: u64,
    },

    /// Updates the fee configuration
    #[account(0, name = "restaking_config")]
    #[account(1, writable, name = "config")]
    #[account(2, name = "ncn")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "restaking_program")]
    SetConfigFees {
        new_fee_wallet: Option<Pubkey>,
        new_block_engine_fee_bps: Option<u64>,
        new_dao_fee_bps: Option<u64>,
        new_ncn_fee_bps: Option<u64>,
        new_ncn_fee_group: Option<u8>,
    },

    /// Sets a new secondary admin for the NCN
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "new_admin")]
    #[account(4, name = "restaking_program")]
    SetNewAdmin {
        role: ConfigAdminRole,
    },

    /// Initializes the weight table for a given NCN epoch
    #[account(0, name = "restaking_config")]
    #[account(1, name = "tracked_mints")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "weight_table")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "restaking_program")]
    #[account(6, name = "system_program")]
    InitializeWeightTable{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    /// Updates the weight table
    #[account(0, name = "ncn")]
    #[account(1, writable, name = "weight_table")]
    #[account(2, signer, name = "weight_table_admin")]
    #[account(3, name = "mint")]
    #[account(4, name = "restaking_program")]
    AdminUpdateWeightTable{
        ncn_epoch: u64,
        weight: u128,
    },

    /// Initializes the Epoch Snapshot
    #[account(0, name = "ncn_config")]
    #[account(1, name = "restaking_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "tracked_mints")]
    #[account(4, name = "weight_table")]
    #[account(5, writable, name = "epoch_snapshot")]
    #[account(6, writable, signer, name = "payer")]
    #[account(7, name = "restaking_program")]
    #[account(8, name = "system_program")]
    InitializeEpochSnapshot{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    /// Initializes the Operator Snapshot
    #[account(0, name = "ncn_config")]
    #[account(1, name = "restaking_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, name = "ncn_operator_state")]
    #[account(5, writable, name = "epoch_snapshot")]
    #[account(6, writable, name = "operator_snapshot")]
    #[account(7, writable, signer, name = "payer")]
    #[account(8, name = "restaking_program")]
    #[account(9, name = "system_program")]
    InitializeOperatorSnapshot{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    /// Initializes the Vault Operator Delegation Snapshot
    #[account(0, name = "ncn_config")]
    #[account(1, name = "restaking_config")]
    #[account(2, name = "tracked_mints")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "vault")]
    #[account(6, name = "vault_ncn_ticket")]
    #[account(7, name = "ncn_vault_ticket")]
    #[account(8, name = "vault_operator_delegation")]
    #[account(9, name = "weight_table")]
    #[account(10, writable, name = "epoch_snapshot")]
    #[account(11, writable, name = "operator_snapshot")]
    #[account(12, name = "vault_program")]
    #[account(13, name = "restaking_program")]
    SnapshotVaultOperatorDelegation{
        first_slot_of_ncn_epoch: Option<u64>,
    },
    /// Registers a mint with the NCN config
    #[account(0, name = "restaking_config")]
    #[account(1, writable, name = "tracked_mints")]
    #[account(2, name = "ncn")]
    #[account(3, name = "weight_table")]
    #[account(4, name = "vault")]
    #[account(5, name = "vault_ncn_ticket")]
    #[account(6, name = "ncn_vault_ticket")]
    #[account(7, name = "restaking_program_id")]
    #[account(8, name = "vault_program_id")]
    RegisterMint,

    /// Initializes the tracked mints account for an NCN
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "tracked_mints")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    InitializeTrackedMints,

    /// Initializes the epoch reward router account for an NCN
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "ballot_box")]
    #[account(3, name = "epoch_reward_router")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "restaking_program_id")]
    #[account(6, name = "system_program")]
    InitializeEpochRewardRouter{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    /// Initializes the epoch reward router account for an NCN
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, name = "ballot_box")]
    #[account(4, name = "operator_epoch_reward_router")]
    #[account(5, writable, signer, name = "payer")]
    #[account(6, name = "restaking_program_id")]
    #[account(7, name = "system_program")]
    InitializeOperatorEpochRewardRouter{
        ncn_fee_group: u8, 
        first_slot_of_ncn_epoch: Option<u64>,
    },

    // restaking_config, ncn, epoch_snapshot, epoch_reward_router, restaking_program
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "epoch_snapshot")]
    #[account(3, name = "epoch_reward_router")]
    #[account(4, name = "restaking_program_id")]
    ProcessEpochRewardPool{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    // restaking_config, ncn, ballot_box, epoch_reward_router, restaking_program
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "ballot_box")]
    #[account(3, name = "epoch_reward_router")]
    #[account(4, name = "restaking_program_id")]
    ProcessEpochRewardBuckets{
        first_slot_of_ncn_epoch: Option<u64>,
    },

    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, name = "operator_snapshot")]
    #[account(4, name = "operator_reward_router")]
    #[account(5, name = "restaking_program_id")]
    ProcessOperatorEpochRewardPool{
        first_slot_of_ncn_epoch: Option<u64>,
    },

}

use borsh::{BorshDeserialize, BorshSerialize};
use shank::ShankInstruction;
use solana_program::pubkey::Pubkey;

use crate::config::ConfigAdminRole;

#[rustfmt::skip]
#[derive(Debug, BorshSerialize, BorshDeserialize, ShankInstruction)]
pub enum TipRouterInstruction {

    // ---------------------------------------------------- //
    //                         GLOBAL                       //
    // ---------------------------------------------------- //
    /// Initialize the config
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "tie_breaker_admin")]
    #[account(4, writable, name = "account_payer")]
    #[account(5, name = "system_program")]
    InitializeConfig {
        epochs_before_stall: u64,
        epochs_after_consensus_before_close: u64,
        valid_slots_after_consensus: u64,
    },

    /// Initializes the vault registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    InitializeVaultRegistry,

    /// Resizes the vault registry account
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    ReallocVaultRegistry,

    /// Registers a vault to the vault registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, name = "vault")]
    #[account(4, name = "ncn_vault_ticket")]
    RegisterVault,

    // ---------------------------------------------------- //
    //                       SNAPSHOT                       //
    // ---------------------------------------------------- //
    /// Initializes the Epoch State
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "account_payer")]
    #[account(5, name = "system_program")]
    InitializeEpochState {
        epoch: u64,
    },

    /// Reallocation of the Epoch State
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "account_payer")]
    #[account(4, name = "system_program")]
    ReallocEpochState {
        epoch: u64,
    },

    /// Initializes the weight table for a given epoch
    #[account(0, name = "epoch_marker")]
    #[account(1, name = "epoch_state")]
    #[account(2, name = "vault_registry")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "weight_table")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    InitializeWeightTable{
        epoch: u64,
    },

    /// Resizes the weight table account
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, writable, name = "weight_table")]
    #[account(3, name = "ncn")]
    #[account(4, name = "vault_registry")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    ReallocWeightTable {
        epoch: u64,
    },

    // Sets the weight table for a given epoch
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "weight_table")]
    #[account(3, name = "switchboard_feed")]
    SwitchboardSetWeight{
        st_mint: Pubkey,
        epoch: u64,
    },


    /// Initializes the Epoch Snapshot
    #[account(0, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "weight_table")]
    #[account(5, writable, name = "epoch_snapshot")]
    #[account(6, writable, name = "account_payer")]
    #[account(7, name = "system_program")]
    InitializeEpochSnapshot{
        epoch: u64,
    },

    /// Initializes the Operator Snapshot
    #[account(0, name = "epoch_marker")]
    #[account(1, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "ncn_operator_state")]
    #[account(6, name = "epoch_snapshot")]
    #[account(7, writable, name = "operator_snapshot")]
    #[account(8, writable, name = "account_payer")]
    #[account(9, name = "system_program")]
    InitializeOperatorSnapshot{
        epoch: u64,
    },

    /// Resizes the operator snapshot account
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, name = "restaking_config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "ncn_operator_state")]
    #[account(6, writable, name = "epoch_snapshot")]
    #[account(7, writable, name = "operator_snapshot")]
    #[account(8, writable, name = "account_payer")]
    #[account(9, name = "system_program")]
    ReallocOperatorSnapshot {
        epoch: u64,
    },
    
    /// Snapshots the vault operator delegation
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, name = "restaking_config")]
    #[account(3, name = "ncn")]
    #[account(4, name = "operator")]
    #[account(5, name = "vault")]
    #[account(6, name = "vault_ncn_ticket")]
    #[account(7, name = "ncn_vault_ticket")]
    #[account(8, name = "vault_operator_delegation")]
    #[account(9, name = "weight_table")]
    #[account(10, writable, name = "epoch_snapshot")]
    #[account(11, writable, name = "operator_snapshot")]
    SnapshotVaultOperatorDelegation{
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                         VOTE                         //
    // ---------------------------------------------------- //
    /// Initializes the ballot box for an NCN
    #[account(0, name = "epoch_marker")]
    #[account(1, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, writable, name = "ballot_box")]
    #[account(4, name = "ncn")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    InitializeBallotBox {
        epoch: u64,
    },

    /// Resizes the ballot box account
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, writable, name = "ballot_box")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "account_payer")]
    #[account(5, name = "system_program")]
    ReallocBallotBox {
        epoch: u64,
    },

    /// Cast a vote for a merkle root
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, writable, name = "ballot_box")]
    #[account(3, name = "ncn")]
    #[account(4, name = "epoch_snapshot")]
    #[account(5, name = "operator_snapshot")]
    #[account(6, name = "operator")]
    #[account(7, signer, name = "operator_voter")]
    CastVote {
        meta_merkle_root: [u8; 32],
        epoch: u64,
    },

    /// Set the merkle root after consensus is reached
    #[account(0, writable, name = "epoch_state")]
    #[account(1, writable, name = "config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "ballot_box")]
    #[account(4, name = "vote_account")]
    #[account(5, writable, name = "tip_distribution_account")]
    #[account(6, name = "tip_distribution_config")]
    #[account(7, name = "tip_distribution_program")]
    SetMerkleRoot {
        proof: Vec<[u8; 32]>,
        merkle_root: [u8; 32],
        max_total_claim: u64,
        max_num_nodes: u64,
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                ROUTE AND DISTRIBUTE                  //
    // ---------------------------------------------------- //
    /// Close an epoch account
    #[account(0, writable, name = "epoch_marker")]
    #[account(1, writable, name = "epoch_state")]
    #[account(2, name = "config")]
    #[account(3, name = "ncn")]
    #[account(4, writable, name = "account_to_close")]
    #[account(5, writable, name = "account_payer")]
    #[account(6, name = "system_program")]
    CloseEpochAccount {
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                        ADMIN                         //
    // ---------------------------------------------------- //
    /// Updates NCN Config parameters
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    AdminSetParameters {
        starting_valid_epoch: Option<u64>,
        epochs_before_stall: Option<u64>,
        epochs_after_consensus_before_close: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
    },


    /// Sets a new secondary admin for the NCN
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "new_admin")]
    AdminSetNewAdmin {
        role: ConfigAdminRole,
    },

    /// Set tie breaker in case of stalled voting
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "config")]
    #[account(2, writable, name = "ballot_box")]
    #[account(3, name = "ncn")]
    #[account(4, signer, name = "tie_breaker_admin")]
    AdminSetTieBreaker {
        meta_merkle_root: [u8; 32],
        epoch: u64,
    },

    /// Sets a weight
    #[account(0, writable, name = "epoch_state")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "weight_table")]
    #[account(3, signer, name = "weight_table_admin")]
    AdminSetWeight{
        st_mint: Pubkey,
        weight: u128,
        epoch: u64,
    },

    /// Registers a new ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "st_mint")]
    #[account(3, writable, name = "vault_registry")]
    #[account(4, signer, writable, name = "admin")]
    AdminRegisterStMint{
        reward_multiplier_bps: u64,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    },

    /// Updates an ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "vault_registry")]
    #[account(3, signer, writable, name = "admin")]
    AdminSetStMint{
        st_mint: Pubkey,
        reward_multiplier_bps: Option<u64>,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    },
}

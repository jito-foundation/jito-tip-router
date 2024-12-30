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
    #[account(2, name = "fee_wallet")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "tie_breaker_admin")]
    #[account(5, name = "restaking_program")]
    #[account(6, name = "system_program")]
    InitializeConfig {
        block_engine_fee_bps: u16,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
        epochs_before_stall: u64,
        valid_slots_after_consensus: u64,
    },

    /// Initializes the vault registry
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    InitializeVaultRegistry,

    /// Resizes the vault registry account
    #[account(0, name = "config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    ReallocVaultRegistry,

    /// Registers a vault to the vault registry
    #[account(0, name = "restaking_config")]
    #[account(1, writable, name = "vault_registry")]
    #[account(2, name = "ncn")]
    #[account(3, name = "vault")]
    #[account(4, name = "vault_ncn_ticket")]
    #[account(5, name = "ncn_vault_ticket")]
    #[account(6, name = "restaking_program_id")]
    #[account(7, name = "vault_program_id")]
    RegisterVault,

    // ---------------------------------------------------- //
    //                       SNAPSHOT                       //
    // ---------------------------------------------------- //
    /// Initializes the weight table for a given epoch
    #[account(0, name = "vault_registry")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "weight_table")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "restaking_program")]
    #[account(5, name = "system_program")]
    InitializeWeightTable{
        epoch: u64,
    },

    /// Resizes the weight table account
    #[account(0, name = "config")]
    #[account(1, writable, name = "weight_table")]
    #[account(2, name = "ncn")]
    #[account(3, name = "vault_registry")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "system_program")]
    ReallocWeightTable {
        epoch: u64,
    },

    // Sets the weight table for a given epoch
    #[account(0, name = "ncn")]
    #[account(1, writable, name = "weight_table")]
    #[account(2, name = "switchboard_feed")]
    SwitchboardSetWeight{
        st_mint: Pubkey,
        epoch: u64,
    },


    /// Initializes the Epoch Snapshot
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "weight_table")]
    #[account(3, writable, name = "epoch_snapshot")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "restaking_program")]
    #[account(6, name = "system_program")]
    InitializeEpochSnapshot{
        epoch: u64,
    },

    /// Initializes the Operator Snapshot
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, name = "ncn_operator_state")]
    #[account(4, writable, name = "epoch_snapshot")]
    #[account(5, writable, name = "operator_snapshot")]
    #[account(6, writable, signer, name = "payer")]
    #[account(7, name = "restaking_program")]
    #[account(8, name = "system_program")]
    InitializeOperatorSnapshot{
        epoch: u64,
    },

    /// Resizes the operator snapshot account
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
    ReallocOperatorSnapshot {
        epoch: u64,
    },
    
    /// Snapshots the vault operator delegation
    #[account(0, name = "config")]
    #[account(1, name = "restaking_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, name = "vault")]
    #[account(5, name = "vault_ncn_ticket")]
    #[account(6, name = "ncn_vault_ticket")]
    #[account(7, name = "vault_operator_delegation")]
    #[account(8, name = "weight_table")]
    #[account(9, writable, name = "epoch_snapshot")]
    #[account(10, writable, name = "operator_snapshot")]
    #[account(11, name = "vault_program")]
    #[account(12, name = "restaking_program")]
    SnapshotVaultOperatorDelegation{
        epoch: u64,
    },

    // ---------------------------------------------------- //
    //                         VOTE                         //
    // ---------------------------------------------------- //
    /// Initializes the ballot box for an NCN
    #[account(0, name = "config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    InitializeBallotBox {
        epoch: u64,
    },

    /// Resizes the ballot box account
    #[account(0, name = "config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    ReallocBallotBox {
        epoch: u64,
    },

    /// Cast a vote for a merkle root
    #[account(0, name = "config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, name = "epoch_snapshot")]
    #[account(4, name = "operator_snapshot")]
    #[account(5, name = "operator")]
    #[account(6, signer, name = "operator_admin")]
    #[account(7, name = "restaking_program")]
    CastVote {
        meta_merkle_root: [u8; 32],
        epoch: u64,
    },

    /// Set the merkle root after consensus is reached
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "ballot_box")]
    #[account(3, name = "vote_account")]
    #[account(4, writable, name = "tip_distribution_account")]
    #[account(5, name = "tip_distribution_config")]
    #[account(6, name = "tip_distribution_program")]
    #[account(7, name = "restaking_program")]
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
    /// Initializes the base reward router
    #[account(0, name = "ncn")]
    #[account(1, writable, name = "base_reward_router")]
    #[account(2, writable, name = "base_reward_receiver")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "restaking_program")]
    #[account(5, name = "system_program")]
    InitializeBaseRewardRouter{
        epoch: u64,
    },

    /// Resizes the base reward router account
    #[account(0, name = "config")]
    #[account(1, writable, name = "base_reward_router")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    ReallocBaseRewardRouter {
        epoch: u64,
    },

    /// Initializes the ncn reward router
    #[account(0, name = "ncn")]
    #[account(1, name = "operator")]
    #[account(2, writable, name = "ncn_reward_router")]
    #[account(3, writable, name = "ncn_reward_receiver")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "restaking_program")]
    #[account(6, name = "system_program")]
    InitializeNcnRewardRouter{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Routes base reward router
    #[account(0, name = "ncn")]
    #[account(1, name = "epoch_snapshot")]
    #[account(2, name = "ballot_box")]
    #[account(3, writable, name = "base_reward_router")]
    #[account(4, writable, name = "base_reward_receiver")]
    #[account(5, name = "restaking_program")]
    RouteBaseRewards{
        max_iterations: u16,
        epoch: u64,
    },

    /// Routes ncn reward router
    #[account(0, name = "ncn")]
    #[account(1, name = "operator")]
    #[account(2, name = "operator_snapshot")]
    #[account(3, writable, name = "ncn_reward_router")]
    #[account(4, writable, name = "ncn_reward_receiver")]
    #[account(5, name = "restaking_program")]
    RouteNcnRewards{
        ncn_fee_group: u8,
        max_iterations: u16,
        epoch: u64,
    },

    /// Distributes base rewards
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "base_reward_router")]
    #[account(3, writable, name = "base_reward_receiver")]
    #[account(4, name = "base_fee_wallet")]
    #[account(5, writable, name = "base_fee_wallet_ata")]
    #[account(6, name = "restaking_program")]
    // Additional accounts for stake pool deposit
    #[account(7, name = "stake_pool_program")]
    #[account(8, writable, name = "stake_pool")]
    #[account(9, name = "stake_pool_withdraw_authority")]
    #[account(10, writable, name = "reserve_stake")]
    #[account(11, writable, name = "manager_fee_account")]
    #[account(12, writable, name = "referrer_pool_tokens_account")]
    #[account(13, writable, name = "pool_mint")]
    #[account(14, name = "token_program")]
    #[account(15, name = "system_program")]
    DistributeBaseRewards{
        base_fee_group: u8,
        epoch: u64,
    },

    /// Distributes base ncn reward routes
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, writable, name = "base_reward_router")]
    #[account(4, writable, name = "base_reward_receiver")]
    #[account(5, writable, name = "ncn_reward_router")]
    #[account(6, writable, name = "ncn_reward_receiver")]
    #[account(7, name = "restaking_program")]
    #[account(8, name = "system_program")]
    DistributeBaseNcnRewardRoute{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Distributes ncn operator rewards
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "operator")]
    #[account(3, writable, name = "operator_ata")]
    #[account(4, writable, name = "ncn_reward_router")]
    #[account(5, writable, name = "ncn_reward_receiver")]
    #[account(6, name = "restaking_program")]
    // Additional accounts for stake pool deposit
    #[account(7, name = "stake_pool_program")]
    #[account(8, writable, name = "stake_pool")]
    #[account(9, name = "stake_pool_withdraw_authority")]
    #[account(10, writable, name = "reserve_stake")]
    #[account(11, writable, name = "manager_fee_account")]
    #[account(12, writable, name = "referrer_pool_tokens_account")]
    #[account(13, writable, name = "pool_mint")]
    #[account(14, name = "token_program")]
    #[account(15, name = "system_program")]
    DistributeNcnOperatorRewards{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Distributes ncn vault rewards
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, writable, name = "vault")]
    #[account(4, writable, name = "vault_ata")]
    #[account(5, writable, name = "ncn_reward_router")]
    #[account(6, writable, name = "ncn_reward_receiver")]
    // Additional accounts for stake pool deposit
    #[account(7, name = "stake_pool_program")]
    #[account(8, writable, name = "stake_pool")]
    #[account(9, name = "stake_pool_withdraw_authority")]
    #[account(10, writable, name = "reserve_stake")]
    #[account(11, writable, name = "manager_fee_account")]
    #[account(12, writable, name = "referrer_pool_tokens_account")]
    #[account(13, writable, name = "pool_mint")]
    #[account(14, name = "token_program")]
    #[account(15, name = "system_program")]
    DistributeNcnVaultRewards{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Claim tips with the program as the payer
    #[account(0, writable, name = "claim_status_payer")]
    #[account(1, name = "tip_distribution_program")]
    #[account(2, name = "config")]
    #[account(3, writable, name = "tip_distribution_account")]
    #[account(4, writable, name = "claim_status")]
    #[account(5, writable, name = "claimant")]
    #[account(6, name = "system_program")]
    ClaimWithPayer {
        proof: Vec<[u8; 32]>,
        amount: u64,
        bump: u8,
    },


    // ---------------------------------------------------- //
    //                        ADMIN                         //
    // ---------------------------------------------------- //
    /// Updates NCN Config parameters
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "restaking_program")]
    AdminSetParameters {
        epochs_before_stall: Option<u64>,
        valid_slots_after_consensus: Option<u64>,
    },

    /// Updates the fee configuration
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "restaking_program")]
    AdminSetConfigFees {
        new_block_engine_fee_bps: Option<u16>,
        base_fee_group: Option<u8>,
        new_base_fee_wallet: Option<Pubkey>,
        new_base_fee_bps: Option<u16>,
        ncn_fee_group: Option<u8>,
        new_ncn_fee_bps: Option<u16>,
    },

    /// Sets a new secondary admin for the NCN
    #[account(0, writable, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, signer, name = "ncn_admin")]
    #[account(3, name = "new_admin")]
    #[account(4, name = "restaking_program")]
    AdminSetNewAdmin {
        role: ConfigAdminRole,
    },

    /// Set tie breaker in case of stalled voting
    #[account(0, name = "config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, signer, name = "tie_breaker_admin")]
    #[account(4, name = "restaking_program")]
    AdminSetTieBreaker {
        meta_merkle_root: [u8; 32],
        epoch: u64,
    },

    /// Sets a weight
    #[account(0, name = "ncn")]
    #[account(1, writable, name = "weight_table")]
    #[account(2, signer, name = "weight_table_admin")]
    #[account(3, name = "restaking_program")]
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
    #[account(5, name = "restaking_program")]
    AdminRegisterStMint{
        ncn_fee_group: u8,
        reward_multiplier_bps: u64,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    },

    /// Updates an ST mint in the Vault Registry
    #[account(0, name = "config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "vault_registry")]
    #[account(3, signer, writable, name = "admin")]
    #[account(4, name = "restaking_program")]
    AdminSetStMint{
        st_mint: Pubkey,
        ncn_fee_group: Option<u8>,
        reward_multiplier_bps: Option<u64>,
        switchboard_feed: Option<Pubkey>,
        no_feed_weight: Option<u128>,
    },
}

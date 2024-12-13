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
        block_engine_fee_bps: u16,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
    },

    /// Initializes the tracked mints account for an NCN
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "tracked_mints")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    InitializeTrackedMints,

    /// Updates the fee configuration
    #[account(0, name = "restaking_config")]
    #[account(1, writable, name = "config")]
    #[account(2, name = "ncn")]
    #[account(3, signer, name = "ncn_admin")]
    #[account(4, name = "restaking_program")]
    SetConfigFees {
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
        epoch: u64,
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
        epoch: u64,
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
        epoch: u64,
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
        epoch: u64,
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


    /// Initializes the base reward router
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, writable, name = "base_reward_router")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "restaking_program")]
    #[account(5, name = "system_program")]
    InitializeBaseRewardRouter{
        epoch: u64,
    },

    /// Initializes the ncn reward router
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, writable, name = "ncn_reward_router")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "restaking_program")]
    #[account(6, name = "system_program")]
    InitializeNcnRewardRouter{
        ncn_fee_group: u8,
        epoch: u64,
    },


    /// Routes base reward router
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "epoch_snapshot")]
    #[account(3, name = "ballot_box")]
    #[account(4, writable, name = "base_reward_router")]
    #[account(5, name = "restaking_program")]
    RouteBaseRewards{
        epoch: u64,
    },

    /// Routes ncn reward router
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn")]
    #[account(2, name = "operator")]
    #[account(3, name = "operator_snapshot")]
    #[account(4, writable, name = "ncn_reward_router")]
    #[account(5, name = "restaking_program")]
    RouteNcnRewards{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Distributes base rewards
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "base_reward_router")]
    #[account(4, writable, name = "base_fee_wallet")]
    #[account(5, name = "restaking_program")]
    DistributeBaseRewards{
        base_fee_group: u8,
        epoch: u64,
    },

    /// Distributes base ncn reward routes
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, writable, name = "base_reward_router")]
    #[account(5, writable, name = "ncn_reward_router")]
    #[account(6, name = "restaking_program")]
    DistributeBaseNcnRewardRoute{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Distributes ncn operator rewards
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, writable, name = "operator")]
    #[account(4, writable, name = "ncn_reward_router")]
    #[account(5, name = "restaking_program")]
    DistributeNcnOperatorRewards{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Distributes ncn vault rewards
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "operator")]
    #[account(4, writable, name = "vault")]
    #[account(5, writable, name = "ncn_reward_router")]
    DistributeNcnVaultRewards{
        ncn_fee_group: u8,
        epoch: u64,
    },

    /// Sets the NCN fee group for a mint
    #[account(0, name = "restaking_config")]
    #[account(1, name = "ncn_config")]
    #[account(2, name = "ncn")]
    #[account(3, name = "weight_table")]
    #[account(4, writable, name = "tracked_mints")]
    #[account(5, signer, writable, name = "admin")]
    #[account(6, name = "restaking_program")]
    SetTrackedMintNcnFeeGroup{
        vault_index: u64,
        ncn_fee_group: u8,
    },

    /// Initializes the ballot box for an NCN
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    InitializeBallotBox {
        epoch: u64,
    },

    /// Cast a vote for a merkle root
    #[account(0, name = "ncn_config")]
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
    #[account(0, writable, name = "ncn_config")]
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

    /// Set tie breaker in case of stalled voting
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, signer, name = "tie_breaker_admin")]
    #[account(4, name = "restaking_program")]
    SetTieBreaker {
        meta_merkle_root: [u8; 32],
        epoch: u64,
    },

    /// Resizes the ballot box account
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "ballot_box")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    ReallocBallotBox {
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

    /// Resizes the base reward router account
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "base_reward_router")]
    #[account(2, name = "ncn")]
    #[account(3, writable, signer, name = "payer")]
    #[account(4, name = "system_program")]
    ReallocBaseRewardRouter {
        epoch: u64,
    },

    /// Resizes the weight table account
    #[account(0, name = "ncn_config")]
    #[account(1, writable, name = "weight_table")]
    #[account(2, name = "ncn")]
    #[account(3, name = "tracked_mints")]
    #[account(4, writable, signer, name = "payer")]
    #[account(5, name = "system_program")]
    ReallocWeightTable {
        epoch: u64,
    },
}

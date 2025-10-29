mod admin_initialize_config;
mod admin_register_st_mint;
mod admin_set_config_fees;
mod admin_set_new_admin;
mod admin_set_parameters;
mod admin_set_st_mint;
mod admin_set_tie_breaker;
mod admin_set_weight;
mod cast_vote;
mod claim_with_payer;
mod close_epoch_account;
mod distribute_base_ncn_reward_route;
mod distribute_base_rewards;
mod distribute_ncn_operator_rewards;
mod distribute_ncn_vault_rewards;
mod initialize_ballot_box;
mod initialize_base_reward_router;
mod initialize_epoch_snapshot;
mod initialize_epoch_state;
mod initialize_ncn_reward_router;
mod initialize_operator_snapshot;
mod initialize_vault_registry;
mod initialize_weight_table;
mod realloc_ballot_box;
mod realloc_base_reward_router;
mod realloc_epoch_state;
mod realloc_operator_snapshot;
mod realloc_vault_registry;
mod realloc_weight_table;
mod register_vault;
mod route_base_rewards;
mod route_ncn_rewards;
mod set_merkle_root;
mod snapshot_vault_operator_delegation;
mod switchboard_set_weight;

use admin_set_new_admin::process_admin_set_new_admin;
use borsh::{BorshDeserialize, BorshSerialize};
use initialize_epoch_state::process_initialize_epoch_state;
use jito_tip_router_core::instruction::TipRouterInstruction;
use realloc_epoch_state::process_realloc_epoch_state;
use solana_program::pubkey;
use solana_program::{
    account_info::AccountInfo,
    declare_id,
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
#[cfg(not(feature = "no-entrypoint"))]
use solana_security_txt::security_txt;
use solana_system_interface::program as system_program;

use crate::{
    admin_initialize_config::process_admin_initialize_config,
    admin_register_st_mint::process_admin_register_st_mint,
    admin_set_config_fees::process_admin_set_config_fees,
    admin_set_parameters::process_admin_set_parameters,
    admin_set_st_mint::process_admin_set_st_mint,
    admin_set_tie_breaker::process_admin_set_tie_breaker,
    admin_set_weight::process_admin_set_weight, cast_vote::process_cast_vote,
    claim_with_payer::process_claim_with_payer, close_epoch_account::process_close_epoch_account,
    distribute_base_ncn_reward_route::process_distribute_base_ncn_reward_route,
    distribute_base_rewards::process_distribute_base_rewards,
    distribute_ncn_operator_rewards::process_distribute_ncn_operator_rewards,
    distribute_ncn_vault_rewards::process_distribute_ncn_vault_rewards,
    initialize_ballot_box::process_initialize_ballot_box,
    initialize_base_reward_router::process_initialize_base_reward_router,
    initialize_epoch_snapshot::process_initialize_epoch_snapshot,
    initialize_ncn_reward_router::process_initialize_ncn_reward_router,
    initialize_operator_snapshot::process_initialize_operator_snapshot,
    initialize_vault_registry::process_initialize_vault_registry,
    initialize_weight_table::process_initialize_weight_table,
    realloc_ballot_box::process_realloc_ballot_box,
    realloc_base_reward_router::process_realloc_base_reward_router,
    realloc_operator_snapshot::process_realloc_operator_snapshot,
    realloc_vault_registry::process_realloc_vault_registry,
    realloc_weight_table::process_realloc_weight_table, register_vault::process_register_vault,
    route_base_rewards::process_route_base_rewards, route_ncn_rewards::process_route_ncn_rewards,
    set_merkle_root::process_set_merkle_root,
    snapshot_vault_operator_delegation::process_snapshot_vault_operator_delegation,
    switchboard_set_weight::process_switchboard_set_weight,
};

declare_id!(env!("TIP_ROUTER_PROGRAM_ID"));

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
        // ---------------------------------------------------- //
        //                         GLOBAL                       //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeConfig {
            block_engine_fee_bps,
            dao_fee_bps,
            default_ncn_fee_bps,
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
        } => {
            msg!("Instruction: InitializeConfig");
            process_admin_initialize_config(
                program_id,
                accounts,
                block_engine_fee_bps,
                dao_fee_bps,
                default_ncn_fee_bps,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
            )
        }
        TipRouterInstruction::InitializeVaultRegistry => {
            msg!("Instruction: InitializeVaultRegistry");
            process_initialize_vault_registry(program_id, accounts)
        }
        TipRouterInstruction::ReallocVaultRegistry => {
            msg!("Instruction: ReallocVaultRegistry");
            process_realloc_vault_registry(program_id, accounts)
        }
        TipRouterInstruction::RegisterVault => {
            msg!("Instruction: RegisterVault");
            process_register_vault(program_id, accounts)
        }

        // ---------------------------------------------------- //
        //                       SNAPSHOT                       //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeEpochState { epoch } => {
            msg!("Instruction: InitializeEpochState");
            process_initialize_epoch_state(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocEpochState { epoch } => {
            msg!("Instruction: ReallocEpochState");
            process_realloc_epoch_state(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeWeightTable { epoch } => {
            msg!("Instruction: InitializeWeightTable");
            process_initialize_weight_table(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocWeightTable { epoch } => {
            msg!("Instruction: ReallocWeightTable");
            process_realloc_weight_table(program_id, accounts, epoch)
        }
        TipRouterInstruction::SwitchboardSetWeight { epoch, st_mint } => {
            msg!("Instruction: SwitchboardSetWeight");
            process_switchboard_set_weight(program_id, accounts, &st_mint, epoch)
        }
        TipRouterInstruction::InitializeEpochSnapshot { epoch } => {
            msg!("Instruction: InitializeEpochSnapshot");
            process_initialize_epoch_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeOperatorSnapshot { epoch } => {
            msg!("Instruction: InitializeOperatorSnapshot");
            process_initialize_operator_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocOperatorSnapshot { epoch } => {
            msg!("Instruction: ReallocOperatorSnapshot");
            process_realloc_operator_snapshot(program_id, accounts, epoch)
        }
        TipRouterInstruction::SnapshotVaultOperatorDelegation { epoch } => {
            msg!("Instruction: SnapshotVaultOperatorDelegation");
            process_snapshot_vault_operator_delegation(program_id, accounts, epoch)
        }

        // ---------------------------------------------------- //
        //                         VOTE                         //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeBallotBox { epoch } => {
            msg!("Instruction: InitializeBallotBox");
            process_initialize_ballot_box(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocBallotBox { epoch } => {
            msg!("Instruction: ReallocBallotBox");
            process_realloc_ballot_box(program_id, accounts, epoch)
        }
        TipRouterInstruction::CastVote {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: CastVote");
            process_cast_vote(program_id, accounts, &meta_merkle_root, epoch)
        }
        TipRouterInstruction::SetMerkleRoot {
            proof,
            merkle_root,
            max_total_claim,
            max_num_nodes,
            epoch,
        } => {
            msg!("Instruction: SetMerkleRoot");
            process_set_merkle_root(
                program_id,
                accounts,
                proof,
                merkle_root,
                max_total_claim,
                max_num_nodes,
                epoch,
            )
        }

        // ---------------------------------------------------- //
        //                ROUTE AND DISTRIBUTE                  //
        // ---------------------------------------------------- //
        TipRouterInstruction::InitializeBaseRewardRouter { epoch } => {
            msg!("Instruction: InitializeBaseRewardRouter");
            process_initialize_base_reward_router(program_id, accounts, epoch)
        }
        TipRouterInstruction::ReallocBaseRewardRouter { epoch } => {
            msg!("Instruction: ReallocBaseRewardRouter");
            process_realloc_base_reward_router(program_id, accounts, epoch)
        }
        TipRouterInstruction::InitializeNcnRewardRouter {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: InitializeNcnRewardRouter");
            process_initialize_ncn_reward_router(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::RouteBaseRewards {
            max_iterations,
            epoch,
        } => {
            msg!("Instruction: RouteBaseRewards");
            process_route_base_rewards(program_id, accounts, max_iterations, epoch)
        }
        TipRouterInstruction::RouteNcnRewards {
            ncn_fee_group,
            max_iterations,
            epoch,
        } => {
            msg!("Instruction: RouteNcnRewards");
            process_route_ncn_rewards(program_id, accounts, ncn_fee_group, max_iterations, epoch)
        }
        TipRouterInstruction::DistributeBaseRewards {
            base_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeBaseRewards");
            process_distribute_base_rewards(program_id, accounts, base_fee_group, epoch)
        }
        TipRouterInstruction::DistributeBaseNcnRewardRoute {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeBaseNcnRewardRoute");
            process_distribute_base_ncn_reward_route(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::DistributeNcnOperatorRewards {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeNcnOperatorRewards");
            process_distribute_ncn_operator_rewards(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::DistributeNcnVaultRewards {
            ncn_fee_group,
            epoch,
        } => {
            msg!("Instruction: DistributeNcnVaultRewards");
            process_distribute_ncn_vault_rewards(program_id, accounts, ncn_fee_group, epoch)
        }
        TipRouterInstruction::ClaimWithPayer {
            proof,
            amount,
            bump,
        } => {
            msg!("Instruction: ClaimWithPayer");
            process_claim_with_payer(program_id, accounts, proof, amount, bump)
        }
        TipRouterInstruction::CloseEpochAccount { epoch } => {
            msg!("Instruction: CloseEpochAccount");
            process_close_epoch_account(program_id, accounts, epoch)
        }

        // ---------------------------------------------------- //
        //                        ADMIN                         //
        // ---------------------------------------------------- //
        TipRouterInstruction::AdminSetParameters {
            starting_valid_epoch,
            epochs_before_stall,
            epochs_after_consensus_before_close,
            valid_slots_after_consensus,
        } => {
            msg!("Instruction: AdminSetParameters");
            process_admin_set_parameters(
                program_id,
                accounts,
                starting_valid_epoch,
                epochs_before_stall,
                epochs_after_consensus_before_close,
                valid_slots_after_consensus,
            )
        }
        TipRouterInstruction::AdminSetConfigFees {
            new_block_engine_fee_bps,
            base_fee_group,
            new_base_fee_wallet,
            new_base_fee_bps,
            ncn_fee_group,
            new_ncn_fee_bps,
            new_priority_fee_distribution_fee_bps,
        } => {
            msg!("Instruction: AdminSetConfigFees");
            process_admin_set_config_fees(
                program_id,
                accounts,
                new_block_engine_fee_bps,
                base_fee_group,
                new_base_fee_wallet,
                new_base_fee_bps,
                ncn_fee_group,
                new_ncn_fee_bps,
                new_priority_fee_distribution_fee_bps,
            )
        }
        TipRouterInstruction::AdminSetNewAdmin { role } => {
            msg!("Instruction: AdminSetNewAdmin");
            process_admin_set_new_admin(program_id, accounts, role)
        }
        TipRouterInstruction::AdminSetTieBreaker {
            meta_merkle_root,
            epoch,
        } => {
            msg!("Instruction: AdminSetTieBreaker");
            process_admin_set_tie_breaker(program_id, accounts, &meta_merkle_root, epoch)
        }
        TipRouterInstruction::AdminSetWeight {
            st_mint,
            weight,
            epoch,
        } => {
            msg!("Instruction: AdminSetWeight");
            process_admin_set_weight(program_id, accounts, &st_mint, epoch, weight)
        }
        TipRouterInstruction::AdminRegisterStMint {
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        } => {
            msg!("Instruction: AdminRegisterStMint");
            process_admin_register_st_mint(
                program_id,
                accounts,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            )
        }
        TipRouterInstruction::AdminSetStMint {
            st_mint,
            ncn_fee_group,
            reward_multiplier_bps,
            switchboard_feed,
            no_feed_weight,
        } => {
            msg!("Instruction: AdminSetStMint");
            process_admin_set_st_mint(
                program_id,
                accounts,
                &st_mint,
                ncn_fee_group,
                reward_multiplier_bps,
                switchboard_feed,
                no_feed_weight,
            )
        }
    }
}

// TODO: Remove all code below when spl-stake-pool-interface is released
pub const fn spl_stake_pool_id() -> Pubkey {
    pubkey!("SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy")
}

// This code was copied from https://github.com/solana-program/stake-pool/blob/main/program/src/instruction.rs#L2019
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum StakePoolInstruction {
    ///   Initializes a new `StakePool`.
    ///
    ///   0. `[w]` New `StakePool` to create.
    ///   1. `[s]` Manager
    ///   2. `[]` Staker
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Uninitialized validator stake list storage account
    ///   5. `[]` Reserve stake account must be initialized, have zero balance,
    ///      and staker / withdrawer authority set to pool withdraw authority.
    ///   6. `[]` Pool token mint. Must have zero supply, owned by withdraw
    ///      authority.
    ///   7. `[]` Pool account to deposit the generated fee for manager.
    ///   8. `[]` Token program id
    ///   9. `[]` (Optional) Deposit authority that must sign all deposits.
    ///      Defaults to the program address generated using
    ///      `find_deposit_authority_program_address`, making deposits
    ///      permissionless.
    Initialize {},

    ///   (Staker only) Adds stake account delegated to validator to the pool's
    ///   list of managed validators.
    ///
    ///   The stake account will have the rent-exempt amount plus
    ///   `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///   It is funded from the stake pool reserve.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[s]` Staker
    ///   2. `[w]` Reserve stake account
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Validator stake list storage account
    ///   5. `[w]` Stake account to add to the pool
    ///   6. `[]` Validator this stake account will be delegated to
    ///   7. `[]` Rent sysvar
    ///   8. `[]` Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. '[]' Stake config sysvar
    ///  11. `[]` System program
    ///  12. `[]` Stake program
    ///
    ///  User data: optional non-zero `u32` seed used for generating the
    ///  validator stake address
    AddValidatorToPool(u32),

    ///   (Staker only) Removes validator from the pool, deactivating its stake
    ///
    ///   Only succeeds if the validator stake account has the minimum of
    ///   `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.   plus the
    /// rent-exempt amount.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[s]` Staker
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator stake list storage account
    ///   4. `[w]` Stake account to remove from the pool
    ///   5. `[w]` Transient stake account, to deactivate if necessary
    ///   6. `[]` Sysvar clock
    ///   7. `[]` Stake program id,
    RemoveValidatorFromPool,

    /// NOTE: This instruction has been deprecated since version 0.7.0. Please
    /// use `DecreaseValidatorStakeWithReserve` instead.
    ///
    /// (Staker only) Decrease active stake on a validator, eventually moving it
    /// to the reserve
    ///
    /// Internally, this instruction splits a validator stake account into its
    /// corresponding transient stake account and deactivates it.
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the transient stake account does not
    /// exist. The amount of lamports to move must be at least rent-exemption
    /// plus `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Canonical stake account to split from
    ///  5. `[w]` Transient stake account to receive split
    ///  6. `[]` Clock sysvar
    ///  7. `[]` Rent sysvar
    ///  8. `[]` System program
    ///  9. `[]` Stake program
    DecreaseValidatorStake {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Increase stake on a validator from the reserve account
    ///
    /// Internally, this instruction splits reserve stake into a transient stake
    /// account and delegate to the appropriate validator.
    /// `UpdateValidatorListBalance` will do the work of merging once it's
    /// ready.
    ///
    /// This instruction only succeeds if the transient stake account does not
    /// exist. The minimum amount to move is rent-exemption plus
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Stake pool reserve stake
    ///  5. `[w]` Transient stake account
    ///  6. `[]` Validator stake account
    ///  7. `[]` Validator vote account to delegate to
    ///  8. '[]' Clock sysvar
    ///  9. '[]' Rent sysvar
    /// 10. `[]` Stake History sysvar
    /// 11. `[]` Stake Config sysvar
    /// 12. `[]` System program
    /// 13. `[]` Stake program
    ///
    /// User data: amount of lamports to increase on the given validator.
    ///
    /// The actual amount split into the transient stake account is:
    /// `lamports + stake_rent_exemption`.
    ///
    /// The rent-exemption of the stake account is withdrawn back to the
    /// reserve after it is merged.
    IncreaseValidatorStake {
        /// amount of lamports to increase on the given validator
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Set the preferred deposit or withdraw stake account for
    /// the stake pool
    ///
    /// In order to avoid users abusing the stake pool as a free conversion
    /// between SOL staked on different validators, the staker can force all
    /// deposits and/or withdraws to go to one chosen account, or unset that
    /// account.
    ///
    /// 0. `[w]` Stake pool
    /// 1. `[s]` Stake pool staker
    /// 2. `[]` Validator list
    ///
    /// Fails if the validator is not part of the stake pool.
    SetPreferredValidator {},

    ///  Updates balances of validator and transient stake accounts in the pool
    ///
    ///  While going through the pairs of validator and transient stake
    ///  accounts, if the transient stake is inactive, it is merged into the
    ///  reserve stake account. If the transient stake is active and has
    ///  matching credits observed, it is merged into the canonical
    ///  validator stake account. In all other states, nothing is done, and
    ///  the balance is simply added to the canonical stake account balance.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[]` Stake pool withdraw authority
    ///  2. `[w]` Validator stake list storage account
    ///  3. `[w]` Reserve stake account
    ///  4. `[]` Sysvar clock
    ///  5. `[]` Sysvar stake history
    ///  6. `[]` Stake program
    ///  7. `..7+2N` [] N pairs of validator and transient stake accounts
    UpdateValidatorListBalance {
        /// Index to start updating on the validator list
        start_index: u32,
        /// If true, don't try merging transient stake accounts into the reserve
        /// or validator stake account.  Useful for testing or if a
        /// particular stake account is in a bad state, but we still
        /// want to update
        no_merge: bool,
    },

    ///   Updates total pool balance based on balances in the reserve and
    ///   validator list
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Validator stake list storage account
    ///   3. `[]` Reserve stake account
    ///   4. `[w]` Account to receive pool fee tokens
    ///   5. `[w]` Pool mint account
    ///   6. `[]` Pool token program
    UpdateStakePoolBalance,

    ///   Cleans up validator stake account entries marked as `ReadyForRemoval`
    ///
    ///   0. `[]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    CleanupRemovedValidatorEntries,

    ///   Deposit some stake into the pool. The output is a "pool" token
    ///   representing ownership into the pool. Inputs are converted to the
    ///   current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[s]/[]` Stake pool deposit authority
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Stake account to join the pool (withdraw authority for the
    ///      stake account should be first set to the stake pool deposit
    ///      authority)
    ///   5. `[w]` Validator stake account for the stake account to be merged
    ///      with
    ///   6. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///   7. `[w]` User account to receive pool tokens
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Account to receive a portion of pool fee tokens as referral
    ///      fees
    ///   10. `[w]` Pool token mint account
    ///   11. '[]' Sysvar clock account
    ///   12. '[]' Sysvar stake history account
    ///   13. `[]` Pool token program id,
    ///   14. `[]` Stake program id,
    DepositStake,

    ///   Withdraw the token from the pool at the current ratio.
    ///
    ///   Succeeds if the stake account has enough SOL to cover the desired
    ///   amount of pool tokens, and if the withdrawal keeps the total
    ///   staked amount above the minimum of rent-exempt amount plus `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///
    ///   When allowing withdrawals, the order of priority goes:
    ///
    ///   * preferred withdraw validator stake account (if set)
    ///   * validator stake accounts
    ///   * transient stake accounts
    ///   * reserve stake account OR totally remove validator stake accounts
    ///
    ///   A user can freely withdraw from a validator stake account, and if they
    ///   are all at the minimum, then they can withdraw from transient stake
    ///   accounts, and if they are all at minimum, then they can withdraw from
    ///   the reserve or remove any validator from the pool.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator or reserve stake account to split
    ///   4. `[w]` Uninitialized stake account to receive withdrawal
    ///   5. `[]` User account to set as a new withdraw authority
    ///   6. `[s]` User transfer authority, for pool token account
    ///   7. `[w]` User account with pool tokens to burn from
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Pool token mint account
    ///  10. `[]` Sysvar clock account (required)
    ///  11. `[]` Pool token program id
    ///  12. `[]` Stake program id,
    ///
    ///  User data: amount of pool tokens to withdraw
    WithdrawStake(u64),

    ///  (Manager only) Update manager
    ///
    ///  0. `[w]` Stake pool
    ///  1. `[s]` Manager
    ///  2. `[s]` New manager
    ///  3. `[]` New manager fee account
    SetManager,

    ///  (Manager only) Update fee
    ///
    ///  0. `[w]` Stake pool
    ///  1. `[s]` Manager
    SetFee {},

    ///  (Manager or staker only) Update staker
    ///
    ///  0. `[w]` Stake pool
    ///  1. `[s]` Manager or current staker
    ///  2. '[]` New staker pubkey
    SetStaker,

    ///   Deposit SOL directly into the pool's reserve account. The output is a
    ///   "pool" token representing ownership into the pool. Inputs are
    ///   converted to the current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Reserve stake account, to deposit SOL
    ///   3. `[s]` Account providing the lamports to be deposited into the pool
    ///   4. `[w]` User account to receive pool tokens
    ///   5. `[w]` Account to receive fee tokens
    ///   6. `[w]` Account to receive a portion of fee as referral fees
    ///   7. `[w]` Pool token mint account
    ///   8. `[]` System program account
    ///   9. `[]` Token program id
    ///  10. `[s]` (Optional) Stake pool sol deposit authority.
    DepositSol(u64),

    ///  (Manager only) Update SOL deposit, stake deposit, or SOL withdrawal
    /// authority.
    ///
    ///  0. `[w]` Stake pool
    ///  1. `[s]` Manager
    ///  2. '[]` New authority pubkey or none
    SetFundingAuthority,

    ///   Withdraw SOL directly from the pool's reserve account. Fails if the
    ///   reserve does not have enough SOL.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[s]` User transfer authority, for pool token account
    ///   3. `[w]` User account to burn pool tokens
    ///   4. `[w]` Reserve stake account, to withdraw SOL
    ///   5. `[w]` Account receiving the lamports from the reserve, must be a
    ///      system account
    ///   6. `[w]` Account to receive pool fee tokens
    ///   7. `[w]` Pool token mint account
    ///   8. '[]' Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. `[]` Stake program account
    ///  11. `[]` Token program id
    ///  12. `[s]` (Optional) Stake pool sol withdraw authority
    WithdrawSol(u64),

    /// Create token metadata for the stake-pool token in the
    /// metaplex-token program
    /// 0. `[]` Stake pool
    /// 1. `[s]` Manager
    /// 2. `[]` Stake pool withdraw authority
    /// 3. `[]` Pool token mint account
    /// 4. `[s, w]` Payer for creation of token metadata account
    /// 5. `[w]` Token metadata account
    /// 6. `[]` Metadata program id
    /// 7. `[]` System program id
    CreateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. `stkSOL`
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },
    /// Update token metadata for the stake-pool token in the
    /// metaplex-token program
    ///
    /// 0. `[]` Stake pool
    /// 1. `[s]` Manager
    /// 2. `[]` Stake pool withdraw authority
    /// 3. `[w]` Token metadata account
    /// 4. `[]` Metadata program id
    UpdateTokenMetadata {
        /// Token name
        name: String,
        /// Token symbol e.g. `stkSOL`
        symbol: String,
        /// URI of the uploaded metadata of the spl-token
        uri: String,
    },

    /// (Staker only) Increase stake on a validator again in an epoch.
    ///
    /// Works regardless if the transient stake account exists.
    ///
    /// Internally, this instruction splits reserve stake into an ephemeral
    /// stake account, activates it, then merges or splits it into the
    /// transient stake account delegated to the appropriate validator.
    /// `UpdateValidatorListBalance` will do the work of merging once it's
    /// ready.
    ///
    /// The minimum amount to move is rent-exemption plus
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Stake pool reserve stake
    ///  5. `[w]` Uninitialized ephemeral stake account to receive stake
    ///  6. `[w]` Transient stake account
    ///  7. `[]` Validator stake account
    ///  8. `[]` Validator vote account to delegate to
    ///  9. '[]' Clock sysvar
    /// 10. `[]` Stake History sysvar
    /// 11. `[]` Stake Config sysvar
    /// 12. `[]` System program
    /// 13. `[]` Stake program
    ///
    /// User data: amount of lamports to increase on the given validator.
    ///
    /// The actual amount split into the transient stake account is:
    /// `lamports + stake_rent_exemption`.
    ///
    /// The rent-exemption of the stake account is withdrawn back to the
    /// reserve after it is merged.
    IncreaseAdditionalValidatorStake {
        /// amount of lamports to increase on the given validator
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
        /// seed used to create ephemeral account.
        ephemeral_stake_seed: u64,
    },

    /// (Staker only) Decrease active stake again from a validator, eventually
    /// moving it to the reserve
    ///
    /// Works regardless if the transient stake account already exists.
    ///
    /// Internally, this instruction:
    ///  * withdraws rent-exempt reserve lamports from the reserve into the
    ///    ephemeral stake
    ///  * splits a validator stake account into an ephemeral stake account
    ///  * deactivates the ephemeral account
    ///  * merges or splits the ephemeral account into the transient stake
    ///    account delegated to the appropriate validator
    ///
    ///  The amount of lamports to move must be at least
    /// `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to fund rent exempt reserve
    ///  5. `[w]` Canonical stake account to split from
    ///  6. `[w]` Uninitialized ephemeral stake account to receive stake
    ///  7. `[w]` Transient stake account
    ///  8. `[]` Clock sysvar
    ///  9. '[]' Stake history sysvar
    /// 10. `[]` System program
    /// 11. `[]` Stake program
    DecreaseAdditionalValidatorStake {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
        /// seed used to create ephemeral account.
        ephemeral_stake_seed: u64,
    },

    /// (Staker only) Decrease active stake on a validator, eventually moving it
    /// to the reserve
    ///
    /// Internally, this instruction:
    /// * withdraws enough lamports to make the transient account rent-exempt
    /// * splits from a validator stake account into a transient stake account
    /// * deactivates the transient stake account
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the transient stake account does not
    /// exist. The amount of lamports to move must be at least rent-exemption
    /// plus `max(crate::MINIMUM_ACTIVE_STAKE,
    /// solana_program::stake::tools::get_minimum_delegation())`.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to fund rent exempt reserve
    ///  5. `[w]` Canonical stake account to split from
    ///  6. `[w]` Transient stake account to receive split
    ///  7. `[]` Clock sysvar
    ///  8. '[]' Stake history sysvar
    ///  9. `[]` System program
    /// 10. `[]` Stake program
    DecreaseValidatorStakeWithReserve {
        /// amount of lamports to split into the transient stake account
        lamports: u64,
        /// seed used to create transient stake account
        transient_stake_seed: u64,
    },

    /// (Staker only) Redelegate active stake on a validator, eventually moving
    /// it to another
    ///
    /// Internally, this instruction splits a validator stake account into its
    /// corresponding transient stake account, redelegates it to an ephemeral
    /// stake account, then merges that stake into the destination transient
    /// stake account.
    ///
    /// In order to rebalance the pool without taking custody, the staker needs
    /// a way of reducing the stake on a stake account. This instruction splits
    /// some amount of stake, up to the total activated stake, from the
    /// canonical validator stake account, into its "transient" stake
    /// account.
    ///
    /// The instruction only succeeds if the source transient stake account and
    /// ephemeral stake account do not exist.
    ///
    /// The amount of lamports to move must be at least rent-exemption plus the
    /// minimum delegation amount. Rent-exemption plus minimum delegation
    /// is required for the destination ephemeral stake account.
    ///
    /// The rent-exemption for the source transient account comes from the stake
    /// pool reserve, if needed.
    ///
    /// The amount that arrives at the destination validator in the end is
    /// `redelegate_lamports - rent_exemption` if the destination transient
    /// account does *not* exist, and `redelegate_lamports` if the destination
    /// transient account already exists. The `rent_exemption` is not activated
    /// when creating the destination transient stake account, but if it already
    /// exists, then the full amount is delegated.
    ///
    ///  0. `[]` Stake pool
    ///  1. `[s]` Stake pool staker
    ///  2. `[]` Stake pool withdraw authority
    ///  3. `[w]` Validator list
    ///  4. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///  5. `[w]` Source canonical stake account to split from
    ///  6. `[w]` Source transient stake account to receive split and be
    ///     redelegated
    ///  7. `[w]` Uninitialized ephemeral stake account to receive redelegation
    ///  8. `[w]` Destination transient stake account to receive ephemeral stake
    ///     by merge
    ///  9. `[]` Destination stake account to receive transient stake after
    ///     activation
    /// 10. `[]` Destination validator vote account
    /// 11. `[]` Clock sysvar
    /// 12. `[]` Stake History sysvar
    /// 13. `[]` Stake Config sysvar
    /// 14. `[]` System program
    /// 15. `[]` Stake program
    #[deprecated(
        since = "2.0.0",
        note = "The stake redelegate instruction used in this will not be enabled."
    )]
    Redelegate {
        /// Amount of lamports to redelegate
        #[allow(dead_code)] // but it's not
        lamports: u64,
        /// Seed used to create source transient stake account
        #[allow(dead_code)] // but it's not
        source_transient_stake_seed: u64,
        /// Seed used to create destination ephemeral account.
        #[allow(dead_code)] // but it's not
        ephemeral_stake_seed: u64,
        /// Seed used to create destination transient stake account. If there is
        /// already transient stake, this must match the current seed, otherwise
        /// it can be anything
        #[allow(dead_code)] // but it's not
        destination_transient_stake_seed: u64,
    },

    ///   Deposit some stake into the pool, with a specified slippage
    ///   constraint. The output is a "pool" token representing ownership
    ///   into the pool. Inputs are converted at the current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[s]/[]` Stake pool deposit authority
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Stake account to join the pool (withdraw authority for the
    ///      stake account should be first set to the stake pool deposit
    ///      authority)
    ///   5. `[w]` Validator stake account for the stake account to be merged
    ///      with
    ///   6. `[w]` Reserve stake account, to withdraw rent exempt reserve
    ///   7. `[w]` User account to receive pool tokens
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Account to receive a portion of pool fee tokens as referral
    ///      fees
    ///   10. `[w]` Pool token mint account
    ///   11. '[]' Sysvar clock account
    ///   12. '[]' Sysvar stake history account
    ///   13. `[]` Pool token program id,
    ///   14. `[]` Stake program id,
    DepositStakeWithSlippage {
        /// Minimum amount of pool tokens that must be received
        minimum_pool_tokens_out: u64,
    },

    ///   Withdraw the token from the pool at the current ratio, specifying a
    ///   minimum expected output lamport amount.
    ///
    ///   Succeeds if the stake account has enough SOL to cover the desired
    ///   amount of pool tokens, and if the withdrawal keeps the total
    ///   staked amount above the minimum of rent-exempt amount plus `max(
    ///     crate::MINIMUM_ACTIVE_STAKE,
    ///     solana_program::stake::tools::get_minimum_delegation()
    ///   )`.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[w]` Validator stake list storage account
    ///   2. `[]` Stake pool withdraw authority
    ///   3. `[w]` Validator or reserve stake account to split
    ///   4. `[w]` Uninitialized stake account to receive withdrawal
    ///   5. `[]` User account to set as a new withdraw authority
    ///   6. `[s]` User transfer authority, for pool token account
    ///   7. `[w]` User account with pool tokens to burn from
    ///   8. `[w]` Account to receive pool fee tokens
    ///   9. `[w]` Pool token mint account
    ///  10. `[]` Sysvar clock account (required)
    ///  11. `[]` Pool token program id
    ///  12. `[]` Stake program id,
    ///
    ///  User data: amount of pool tokens to withdraw
    WithdrawStakeWithSlippage {
        /// Pool tokens to burn in exchange for lamports
        pool_tokens_in: u64,
        /// Minimum amount of lamports that must be received
        minimum_lamports_out: u64,
    },

    ///   Deposit SOL directly into the pool's reserve account, with a
    ///   specified slippage constraint. The output is a "pool" token
    ///   representing ownership into the pool. Inputs are converted at the
    ///   current ratio.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Reserve stake account, to deposit SOL
    ///   3. `[s]` Account providing the lamports to be deposited into the pool
    ///   4. `[w]` User account to receive pool tokens
    ///   5. `[w]` Account to receive fee tokens
    ///   6. `[w]` Account to receive a portion of fee as referral fees
    ///   7. `[w]` Pool token mint account
    ///   8. `[]` System program account
    ///   9. `[]` Token program id
    ///  10. `[s]` (Optional) Stake pool sol deposit authority.
    DepositSolWithSlippage {
        /// Amount of lamports to deposit into the reserve
        lamports_in: u64,
        /// Minimum amount of pool tokens that must be received
        minimum_pool_tokens_out: u64,
    },

    ///   Withdraw SOL directly from the pool's reserve account. Fails if the
    ///   reserve does not have enough SOL or if the slippage constraint is not
    ///   met.
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[s]` User transfer authority, for pool token account
    ///   3. `[w]` User account to burn pool tokens
    ///   4. `[w]` Reserve stake account, to withdraw SOL
    ///   5. `[w]` Account receiving the lamports from the reserve, must be a
    ///      system account
    ///   6. `[w]` Account to receive pool fee tokens
    ///   7. `[w]` Pool token mint account
    ///   8. '[]' Clock sysvar
    ///   9. '[]' Stake history sysvar
    ///  10. `[]` Stake program account
    ///  11. `[]` Token program id
    ///  12. `[s]` (Optional) Stake pool sol withdraw authority
    WithdrawSolWithSlippage {
        /// Pool tokens to burn in exchange for lamports
        pool_tokens_in: u64,
        /// Minimum amount of lamports that must be received
        minimum_lamports_out: u64,
    },
}

#[allow(clippy::too_many_arguments)]
pub fn deposit_sol(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    lamports_in: u64,
) -> Instruction {
    deposit_sol_internal(
        program_id,
        stake_pool,
        stake_pool_withdraw_authority,
        reserve_stake_account,
        lamports_from,
        pool_tokens_to,
        manager_fee_account,
        referrer_pool_tokens_account,
        pool_mint,
        token_program_id,
        None,
        lamports_in,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
fn deposit_sol_internal(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    reserve_stake_account: &Pubkey,
    lamports_from: &Pubkey,
    pool_tokens_to: &Pubkey,
    manager_fee_account: &Pubkey,
    referrer_pool_tokens_account: &Pubkey,
    pool_mint: &Pubkey,
    token_program_id: &Pubkey,
    sol_deposit_authority: Option<&Pubkey>,
    lamports_in: u64,
    minimum_pool_tokens_out: Option<u64>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*reserve_stake_account, false),
        AccountMeta::new(*lamports_from, true),
        AccountMeta::new(*pool_tokens_to, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*referrer_pool_tokens_account, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(sol_deposit_authority) = sol_deposit_authority {
        accounts.push(AccountMeta::new_readonly(*sol_deposit_authority, true));
    }
    if let Some(minimum_pool_tokens_out) = minimum_pool_tokens_out {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::DepositSolWithSlippage {
                lamports_in,
                minimum_pool_tokens_out,
            })
            .unwrap(),
        }
    } else {
        Instruction {
            program_id: *program_id,
            accounts,
            data: borsh::to_vec(&StakePoolInstruction::DepositSol(lamports_in)).unwrap(),
        }
    }
}

/// Seed for withdraw authority seed
const AUTHORITY_WITHDRAW: &[u8] = b"withdraw";

pub fn find_withdraw_authority_program_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[stake_pool_address.as_ref(), AUTHORITY_WITHDRAW],
        program_id,
    )
}

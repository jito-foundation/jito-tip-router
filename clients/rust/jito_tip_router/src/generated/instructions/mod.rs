//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

pub(crate) mod r#admin_update_weight_table;
pub(crate) mod r#cast_vote;
pub(crate) mod r#claim_with_payer;
pub(crate) mod r#distribute_base_ncn_reward_route;
pub(crate) mod r#distribute_base_rewards;
pub(crate) mod r#distribute_ncn_operator_rewards;
pub(crate) mod r#distribute_ncn_vault_rewards;
pub(crate) mod r#initialize_ballot_box;
pub(crate) mod r#initialize_base_reward_router;
pub(crate) mod r#initialize_epoch_snapshot;
pub(crate) mod r#initialize_n_c_n_config;
pub(crate) mod r#initialize_ncn_reward_router;
pub(crate) mod r#initialize_operator_snapshot;
pub(crate) mod r#initialize_tracked_mints;
pub(crate) mod r#initialize_weight_table;
pub(crate) mod r#realloc_ballot_box;
pub(crate) mod r#realloc_base_reward_router;
pub(crate) mod r#realloc_operator_snapshot;
pub(crate) mod r#realloc_weight_table;
pub(crate) mod r#register_mint;
pub(crate) mod r#route_base_rewards;
pub(crate) mod r#route_ncn_rewards;
pub(crate) mod r#set_config_fees;
pub(crate) mod r#set_merkle_root;
pub(crate) mod r#set_new_admin;
pub(crate) mod r#set_tie_breaker;
pub(crate) mod r#set_tracked_mint_ncn_fee_group;
pub(crate) mod r#snapshot_vault_operator_delegation;

pub use self::{
    r#admin_update_weight_table::*, r#cast_vote::*, r#claim_with_payer::*,
    r#distribute_base_ncn_reward_route::*, r#distribute_base_rewards::*,
    r#distribute_ncn_operator_rewards::*, r#distribute_ncn_vault_rewards::*,
    r#initialize_ballot_box::*, r#initialize_base_reward_router::*, r#initialize_epoch_snapshot::*,
    r#initialize_n_c_n_config::*, r#initialize_ncn_reward_router::*,
    r#initialize_operator_snapshot::*, r#initialize_tracked_mints::*, r#initialize_weight_table::*,
    r#realloc_ballot_box::*, r#realloc_base_reward_router::*, r#realloc_operator_snapshot::*,
    r#realloc_weight_table::*, r#register_mint::*, r#route_base_rewards::*, r#route_ncn_rewards::*,
    r#set_config_fees::*, r#set_merkle_root::*, r#set_new_admin::*, r#set_tie_breaker::*,
    r#set_tracked_mint_ncn_fee_group::*, r#snapshot_vault_operator_delegation::*,
};

//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

pub(crate) mod r#admin_update_weight_table;
pub(crate) mod r#initialize_epoch_snapshot;
pub(crate) mod r#initialize_n_c_n_config;
pub(crate) mod r#initialize_operator_snapshot;
pub(crate) mod r#initialize_tracked_mints;
pub(crate) mod r#initialize_vault_operator_delegation_snapshot;
pub(crate) mod r#initialize_weight_table;
pub(crate) mod r#register_mint;
pub(crate) mod r#set_config_fees;
pub(crate) mod r#set_new_admin;

pub use self::{
    r#admin_update_weight_table::*, r#initialize_epoch_snapshot::*, r#initialize_n_c_n_config::*,
    r#initialize_operator_snapshot::*, r#initialize_tracked_mints::*,
    r#initialize_vault_operator_delegation_snapshot::*, r#initialize_weight_table::*,
    r#register_mint::*, r#set_config_fees::*, r#set_new_admin::*,
};

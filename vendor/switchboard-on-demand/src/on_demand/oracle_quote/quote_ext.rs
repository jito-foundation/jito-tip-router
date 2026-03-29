//! Extension trait for convenient Anchor account wrapper methods
//!
//! This module provides the `SwitchboardQuoteExt` trait which adds utility methods
//! to Anchor's `Account` and `InterfaceAccount` wrappers for `SwitchboardQuote`.

use crate::on_demand::oracle_quote::feed_info::PackedFeedInfo;
use crate::on_demand::oracle_quote::quote_account::SwitchboardQuote;

/// Extension trait to provide convenient methods for Anchor account wrappers
#[cfg(feature = "anchor")]
pub trait SwitchboardQuoteExt<'a> {
    /// Get the canonical oracle account key for this quote's feeds
    fn canonical_key(&self, queue: &anchor_lang::solana_program::pubkey::Pubkey) -> anchor_lang::solana_program::pubkey::Pubkey;

    /// Get the owner of the account
    fn owner(&self) -> &anchor_lang::solana_program::pubkey::Pubkey;

    /// Get feeds from the oracle quote
    fn feeds(&self) -> &[PackedFeedInfo];

    /// Write oracle quote data from an ED25519 instruction with slot validation
    fn write_from_ix<'b, I>(&mut self, ix_sysvar: I, curr_slot: u64, instruction_index: usize)
    where
        I: AsRef<anchor_lang::prelude::AccountInfo<'b>>;

    /// Write oracle quote data from an ED25519 instruction without slot validation.
    ///
    /// # ⚠️ WARNING ⚠️
    /// **This method bypasses critical security validations. See [`OracleQuote::write_from_ix_unchecked`] for detailed security warnings.**
    ///
    /// [`OracleQuote::write_from_ix_unchecked`]: crate::on_demand::oracle_quote::OracleQuote::write_from_ix_unchecked
    fn write_from_ix_unchecked<'b, I>(&mut self, ix_sysvar: I, instruction_index: usize)
    where
        I: AsRef<anchor_lang::prelude::AccountInfo<'b>>;

    /// Check if the account is initialized by checking the last 4 bytes are SBOD
    fn is_initialized(&self) -> bool;

    /// if !is_initialized, return if the new quotes canonical key matches the account key
    /// else just check if the account key match the new quotes
    fn keys_match(&self, quote: &crate::on_demand::oracle_quote::OracleQuote) -> bool;
}

/// Macro to implement SwitchboardQuoteExt for Anchor account wrapper types
///
/// This macro generates identical implementations for different account wrapper types
/// (Account, InterfaceAccount) to avoid code duplication.
#[cfg(feature = "anchor")]
macro_rules! impl_switchboard_quote_ext {
    ($wrapper:ty) => {
        impl<'info> SwitchboardQuoteExt<'info> for $wrapper {
            fn canonical_key(&self, queue: &anchor_lang::solana_program::pubkey::Pubkey) -> anchor_lang::solana_program::pubkey::Pubkey {
                use anchor_lang::ToAccountInfo;
                (**self).canonical_key(queue, self.to_account_info().owner)
            }

            fn owner(&self) -> &anchor_lang::solana_program::pubkey::Pubkey {
                use anchor_lang::ToAccountInfo;
                self.to_account_info().owner
            }

            fn feeds(&self) -> &[PackedFeedInfo] {
                self.feeds.as_slice()
            }

            fn write_from_ix<'b, I>(&mut self, ix_sysvar: I, curr_slot: u64, instruction_index: usize)
            where
                I: AsRef<anchor_lang::prelude::AccountInfo<'b>>,
            {
                let ix_sysvar = ix_sysvar.as_ref();
                let data = crate::Instructions::extract_ix_data(ix_sysvar, instruction_index);
                use anchor_lang::ToAccountInfo;
                crate::on_demand::oracle_quote::OracleQuote::write(curr_slot, data, &self.queue, &self.to_account_info());
            }

            fn write_from_ix_unchecked<'b, I>(&mut self, ix_sysvar: I, instruction_index: usize)
            where
                I: AsRef<anchor_lang::prelude::AccountInfo<'b>>,
            {
                let ix_sysvar = ix_sysvar.as_ref();
                let data = crate::Instructions::extract_ix_data(ix_sysvar, instruction_index);
                use anchor_lang::ToAccountInfo;
                crate::on_demand::oracle_quote::OracleQuote::write_unchecked(data, &self.queue, &self.to_account_info());
            }

            fn is_initialized(&self) -> bool {
                const TAIL_DISCRIMINATOR: [u8; 4] = *b"SBOD";
                self.tail_discriminator == TAIL_DISCRIMINATOR
            }

            fn keys_match(&self, quote: &crate::on_demand::oracle_quote::OracleQuote) -> bool {
                if !self.is_initialized() {
                    return false;
                }
                let own_feeds = &self.feeds;
                let other_feeds = quote.feeds();
                if own_feeds.len() != other_feeds.len() {
                    return false;
                }
                for i in 0..own_feeds.len() {
                    if !crate::check_pubkey_eq(&own_feeds[i].feed_id, other_feeds[i].feed_id()) {
                        return false;
                    }
                }
                true
            }
        }
    };
}

// Implement the trait for both Account and InterfaceAccount wrapper types
#[cfg(feature = "anchor")]
impl_switchboard_quote_ext!(anchor_lang::prelude::Account<'info, SwitchboardQuote>);

#[cfg(feature = "anchor")]
impl_switchboard_quote_ext!(anchor_lang::prelude::InterfaceAccount<'info, SwitchboardQuote>);

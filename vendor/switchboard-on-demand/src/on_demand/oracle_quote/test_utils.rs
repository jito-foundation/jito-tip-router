//! Test utilities for creating mock SwitchboardQuote accounts
//!
//! This module provides builder methods and utilities for creating SwitchboardQuote
//! instances with custom feed data for testing purposes, particularly useful with
//! litesvm and other local testing environments.
//!
//! # Example
//!
//! ```rust
//! # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
//! # use switchboard_on_demand::Pubkey;
//! // Create a quote with mock BTC/USD data
//! let btc_feed_id = [0x42; 32]; // Example feed ID
//! let queue_key = Pubkey::new_unique();
//!
//! let quote = QuoteBuilder::new(queue_key)
//!     .add_feed(&btc_feed_id, 95000.0)  // BTC at $95,000
//!     .slot(1000)
//!     .build()
//!     .expect("Failed to build quote");
//!
//! // Serialize for use in tests
//! let account_data = quote.to_account_data().expect("Failed to serialize");
//! ```

use crate::on_demand::oracle_quote::{
    feed_info::{PackedFeedInfo, PackedQuoteHeader},
    quote_account::SwitchboardQuote,
};
use crate::smallvec::{SmallVec, U16Prefix, U8Prefix};
use crate::Pubkey;
use rust_decimal::prelude::*;

/// Precision constant for feed values (18 decimal places)
const PRECISION: u32 = 18;

/// Converts a decimal price to the scaled i128 value used in PackedFeedInfo
///
/// # Arguments
/// * `value` - The decimal value (e.g., 95000.0 for $95,000)
///
/// # Returns
/// The scaled i128 value
fn scale_value(value: f64) -> i128 {
    let decimal = Decimal::from_f64_retain(value).expect("Invalid decimal value");
    let scaled = decimal * Decimal::from(10_i128.pow(PRECISION));
    scaled.to_i128().expect("Value too large to scale")
}

/// Builder for creating SwitchboardQuote instances for testing
///
/// This builder provides a convenient API for constructing mock oracle quotes
/// with custom feed data, making it easy to test oracle-dependent code locally.
///
/// # Example
///
/// ```rust
/// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
/// # use switchboard_on_demand::Pubkey;
/// let quote = QuoteBuilder::new(Pubkey::new_unique())
///     .add_feed_hex("0xef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f", 95000.0)
///     .add_feed_hex("0x84c2dde9633d93d1bcad84e7dc41c9d56578b7ec52fabedc1f335d673df0a7c1", 3500.0)
///     .slot(1000)
///     .build()
///     .unwrap();
/// ```
pub struct QuoteBuilder {
    queue: Pubkey,
    feeds: Vec<(Vec<u8>, f64, u8)>, // (feed_id, value, min_samples)
    slot: u64,
    version: u8,
}

impl QuoteBuilder {
    /// Create a new QuoteBuilder for the given queue
    ///
    /// # Arguments
    /// * `queue` - The queue pubkey this quote belongs to
    pub fn new(queue: Pubkey) -> Self {
        Self {
            queue,
            feeds: Vec::new(),
            slot: 0,
            version: 1,
        }
    }

    /// Add a feed with raw feed ID bytes
    ///
    /// # Arguments
    /// * `feed_id` - 32-byte feed identifier
    /// * `value` - Feed value (e.g., 95000.0 for $95,000)
    ///
    /// # Example
    /// ```rust
    /// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
    /// # use switchboard_on_demand::Pubkey;
    /// let feed_id = [0x42; 32];
    /// let quote = QuoteBuilder::new(Pubkey::new_unique())
    ///     .add_feed(&feed_id, 95000.0)
    ///     .build()
    ///     .unwrap();
    /// ```
    pub fn add_feed(mut self, feed_id: &[u8; 32], value: f64) -> Self {
        self.feeds.push((feed_id.to_vec(), value, 1));
        self
    }

    /// Add a feed with a hexadecimal feed ID string
    ///
    /// # Arguments
    /// * `feed_id_hex` - Hexadecimal string with or without "0x" prefix
    /// * `value` - Feed value (e.g., 95000.0 for $95,000)
    ///
    /// # Example
    /// ```rust
    /// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
    /// # use switchboard_on_demand::Pubkey;
    /// let quote = QuoteBuilder::new(Pubkey::new_unique())
    ///     .add_feed_hex("0xef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f", 95000.0)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[cfg(feature = "client")]
    pub fn add_feed_hex(mut self, feed_id_hex: &str, value: f64) -> Self {
        use hex;
        let hex_str = feed_id_hex.strip_prefix("0x").unwrap_or(feed_id_hex);
        let feed_id = hex::decode(hex_str).expect("Invalid hex string");
        if feed_id.len() != 32 {
            panic!("Feed ID must be exactly 32 bytes");
        }
        self.feeds.push((feed_id, value, 1));
        self
    }

    /// Add a feed with a hexadecimal feed ID string and custom minimum samples
    ///
    /// # Arguments
    /// * `feed_id_hex` - Hexadecimal string with or without "0x" prefix
    /// * `value` - Feed value (e.g., 95000.0 for $95,000)
    /// * `min_samples` - Minimum number of oracle samples required
    ///
    /// # Example
    /// ```rust
    /// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
    /// # use switchboard_on_demand::Pubkey;
    /// let quote = QuoteBuilder::new(Pubkey::new_unique())
    ///     .add_feed_with_samples("0xef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f", 95000.0, 3)
    ///     .build()
    ///     .unwrap();
    /// ```
    #[cfg(feature = "client")]
    pub fn add_feed_with_samples(
        mut self,
        feed_id_hex: &str,
        value: f64,
        min_samples: u8,
    ) -> Self {
        use hex;
        let hex_str = feed_id_hex.strip_prefix("0x").unwrap_or(feed_id_hex);
        let feed_id = hex::decode(hex_str).expect("Invalid hex string");
        if feed_id.len() != 32 {
            panic!("Feed ID must be exactly 32 bytes");
        }
        self.feeds.push((feed_id, value, min_samples));
        self
    }

    /// Set the slot number for this quote
    ///
    /// # Arguments
    /// * `slot` - Slot number for freshness validation
    pub fn slot(mut self, slot: u64) -> Self {
        self.slot = slot;
        self
    }

    /// Set the version byte for this quote
    ///
    /// # Arguments
    /// * `version` - Version byte (default: 1)
    pub fn version(mut self, version: u8) -> Self {
        self.version = version;
        self
    }

    /// Build the SwitchboardQuote instance
    ///
    /// This creates a minimal valid quote suitable for testing. It includes:
    /// - Empty signatures (verification disabled in test mode)
    /// - Mock slot hash
    /// - The feeds you've added
    /// - Minimal oracle indices
    ///
    /// # Returns
    /// Result containing the built SwitchboardQuote or an error message
    ///
    /// # Example
    /// ```rust
    /// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
    /// # use switchboard_on_demand::Pubkey;
    /// let quote = QuoteBuilder::new(Pubkey::new_unique())
    ///     .add_feed_hex("0xef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f", 95000.0)
    ///     .slot(1000)
    ///     .build()
    ///     .expect("Failed to build quote");
    /// ```
    pub fn build(self) -> Result<SwitchboardQuote, &'static str> {
        if self.feeds.is_empty() {
            return Err("At least one feed is required");
        }

        // Create packed feed infos
        let mut feeds_vec = Vec::new();
        for (feed_id_vec, value, min_samples) in self.feeds {
            let mut feed_id = [0u8; 32];
            feed_id.copy_from_slice(&feed_id_vec);
            feeds_vec.push(PackedFeedInfo {
                feed_id,
                feed_value: scale_value(value),
                min_oracle_samples: min_samples,
            });
        }

        let feeds: SmallVec<PackedFeedInfo, U8Prefix> =
            feeds_vec.try_into().map_err(|_| "Too many feeds")?;

        // Create mock signatures (one dummy signature for testing)
        use crate::on_demand::oracle_quote::quote_account::OracleSignature;
        use crate::sysvar::ed25519_sysvar::Ed25519SignatureOffsets;

        let dummy_sig = OracleSignature {
            offsets: Ed25519SignatureOffsets {
                signature_offset: 0,
                signature_instruction_index: 0,
                public_key_offset: 0,
                public_key_instruction_index: 0,
                message_data_offset: 0,
                message_data_size: 0,
                message_instruction_index: 0,
            },
            pubkey: Pubkey::new_from_array([0u8; 32]),
            signature: [0u8; 64],
        };

        let signatures: SmallVec<OracleSignature, U16Prefix> =
            vec![dummy_sig].try_into().map_err(|_| "Failed to create signatures")?;

        // Create mock quote header with a dummy slot hash
        let quote_header = PackedQuoteHeader {
            signed_slothash: [0u8; 32],
        };

        // Create oracle indices (one oracle for simplicity)
        let oracle_idxs: SmallVec<u8, U8Prefix> =
            vec![0].try_into().map_err(|_| "Failed to create oracle indices")?;

        Ok(SwitchboardQuote {
            queue: self.queue,
            signatures,
            quote_header,
            feeds,
            oracle_idxs,
            slot: self.slot,
            version: self.version,
            tail_discriminator: *b"SBOD",
        })
    }
}

impl SwitchboardQuote {
    /// Serialize the quote to account data format suitable for litesvm/testing
    ///
    /// This creates the complete account data including the 8-byte discriminator
    /// that would be present in an actual on-chain account.
    ///
    /// # Returns
    /// Result containing the serialized account data or an error
    ///
    /// # Example
    /// ```rust
    /// # use switchboard_on_demand::on_demand::oracle_quote::test_utils::QuoteBuilder;
    /// # use switchboard_on_demand::Pubkey;
    /// let quote = QuoteBuilder::new(Pubkey::new_unique())
    ///     .add_feed_hex("0xef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f", 95000.0)
    ///     .build()
    ///     .unwrap();
    ///
    /// let account_data = quote.to_account_data().expect("Failed to serialize");
    /// // Use account_data in litesvm or other tests
    /// ```
    #[cfg(feature = "anchor")]
    pub fn to_account_data(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use crate::on_demand::oracle_quote::quote_account::QUOTE_DISCRIMINATOR;
        use borsh::BorshSerialize;

        let mut data = Vec::new();

        // Write discriminator (8 bytes)
        data.extend_from_slice(QUOTE_DISCRIMINATOR);

        // Write queue pubkey (32 bytes)
        data.extend_from_slice(self.queue.as_ref());

        // Create ED25519 instruction format data
        let mut ed25519_data = Vec::new();

        let num_sigs = self.signatures.len() as u8;

        // Header: num_signatures + padding
        ed25519_data.push(num_sigs);
        ed25519_data.push(0u8); // padding

        // Calculate offsets
        let header_size = 2; // num_signatures(1) + padding(1)
        let offsets_size = 14 * num_sigs as usize; // 14 bytes per signature

        // Pubkeys start right after all offsets
        let pubkeys_offset = header_size + offsets_size;

        // Signatures start after all pubkeys (32 bytes each)
        let signatures_offset = pubkeys_offset + (32 * num_sigs as usize);

        // Message starts after all signatures (64 bytes each)
        let message_offset = signatures_offset + (64 * num_sigs as usize);

        // Serialize message data first to get its size
        let mut message_data = Vec::new();
        self.quote_header.serialize(&mut message_data)?;
        for feed in self.feeds.iter() {
            feed.serialize(&mut message_data)?;
        }
        let message_size = message_data.len() as u16;

        // Write offsets with correct pointers
        for i in 0..num_sigs {
            let sig_offset = signatures_offset + (i as usize * 64);
            let pubkey_offset = pubkeys_offset + (i as usize * 32);

            // Ed25519SignatureOffsets struct (14 bytes)
            ed25519_data.extend_from_slice(&(sig_offset as u16).to_le_bytes());      // signature_offset
            ed25519_data.extend_from_slice(&0u16.to_le_bytes());                     // signature_instruction_index
            ed25519_data.extend_from_slice(&(pubkey_offset as u16).to_le_bytes());   // public_key_offset
            ed25519_data.extend_from_slice(&0u16.to_le_bytes());                     // public_key_instruction_index
            ed25519_data.extend_from_slice(&(message_offset as u16).to_le_bytes());  // message_data_offset
            ed25519_data.extend_from_slice(&message_size.to_le_bytes());             // message_data_size
            ed25519_data.extend_from_slice(&0u16.to_le_bytes());                     // message_instruction_index
        }

        // Write all pubkeys
        for sig in self.signatures.iter() {
            ed25519_data.extend_from_slice(&sig.pubkey.to_bytes());
        }

        // Write all signatures
        for sig in self.signatures.iter() {
            ed25519_data.extend_from_slice(&sig.signature);
        }

        // Write message data
        ed25519_data.extend_from_slice(&message_data);

        // Write suffix (oracle_idxs + slot + version + tail discriminator)
        for idx in self.oracle_idxs.iter() {
            ed25519_data.push(*idx);
        }
        ed25519_data.extend_from_slice(&self.slot.to_le_bytes());
        ed25519_data.push(self.version);
        ed25519_data.extend_from_slice(&self.tail_discriminator);

        // Write u16 length prefix for ed25519_data
        let len = ed25519_data.len() as u16;
        data.extend_from_slice(&len.to_le_bytes());

        // Write the ed25519_data
        data.extend_from_slice(&ed25519_data);

        Ok(data)
    }

    /// Serialize the quote to account data format (without anchor feature)
    ///
    /// # Returns
    /// Result containing the serialized account data or an error
    #[cfg(not(feature = "anchor"))]
    pub fn to_account_data(&self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        use borsh::BorshSerialize;
        use crate::on_demand::oracle_quote::quote_account::QUOTE_DISCRIMINATOR;
        let mut data = Vec::new();

        // Write discriminator
        data.extend_from_slice(QUOTE_DISCRIMINATOR);

        // Write the quote data
        self.serialize(&mut data)?;

        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_builder_basic() {
        let queue = Pubkey::new_unique();
        let feed_id = [0x42; 32];

        let quote = QuoteBuilder::new(queue)
            .add_feed(&feed_id, 95000.0)
            .slot(1000)
            .build()
            .expect("Failed to build quote");

        assert_eq!(quote.queue, queue);
        assert_eq!(quote.feeds.len(), 1);
        assert_eq!(quote.feeds[0].feed_id, feed_id);
        assert_eq!(quote.slot, 1000);
    }

    #[test]
    #[cfg(feature = "client")]
    fn test_quote_builder_hex_feed() {
        let queue = Pubkey::new_unique();
        let feed_id_hex = "ef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f";

        let quote = QuoteBuilder::new(queue)
            .add_feed_hex(feed_id_hex, 95000.0)
            .build()
            .expect("Failed to build quote");

        assert_eq!(quote.feeds.len(), 1);
        assert_eq!(quote.feeds[0].hex_id(), format!("0x{}", feed_id_hex));
    }

    #[test]
    #[cfg(feature = "client")]
    fn test_quote_builder_multiple_feeds() {
        let queue = Pubkey::new_unique();

        let quote = QuoteBuilder::new(queue)
            .add_feed_hex(
                "ef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f",
                95000.0,
            )
            .add_feed_hex(
                "84c2dde9633d93d1bcad84e7dc41c9d56578b7ec52fabedc1f335d673df0a7c1",
                3500.0,
            )
            .slot(2000)
            .build()
            .expect("Failed to build quote");

        assert_eq!(quote.feeds.len(), 2);
        assert_eq!(quote.slot, 2000);
    }

    #[test]
    fn test_scale_value() {
        // Test that values are scaled correctly
        let scaled = scale_value(95000.0);
        let decimal = Decimal::from_i128_with_scale(scaled, PRECISION);
        assert_eq!(decimal.normalize().to_string(), "95000");

        let scaled2 = scale_value(3.14159);
        let decimal2 = Decimal::from_i128_with_scale(scaled2, PRECISION);
        // Check that the value is approximately correct (floating point precision limits)
        assert!(decimal2.to_string().starts_with("3.14158") || decimal2.to_string().starts_with("3.14159"));
    }

    #[test]
    fn test_quote_builder_requires_feeds() {
        let queue = Pubkey::new_unique();
        let result = QuoteBuilder::new(queue).build();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "At least one feed is required");
    }

    #[test]
    #[cfg(all(feature = "anchor", feature = "client"))]
    fn test_quote_serialization() {
        use crate::on_demand::oracle_quote::quote_account::QUOTE_DISCRIMINATOR;
        let queue = Pubkey::new_unique();
        let quote = QuoteBuilder::new(queue)
            .add_feed_hex(
                "ef0d8b6fcd0104e3e75096912fc8e1e432893da4f18faedaacca7e5875da620f",
                95000.0,
            )
            .build()
            .expect("Failed to build quote");

        let account_data = quote.to_account_data().expect("Failed to serialize");

        // Check that discriminator is present
        assert_eq!(&account_data[0..8], QUOTE_DISCRIMINATOR);
        // Check minimum size
        assert!(account_data.len() >= 40); // discriminator + queue + minimal data
    }
}

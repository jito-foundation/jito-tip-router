/// Core Crossbar protocol implementations and client functionality
pub mod crossbar;
pub use crossbar::*;

/// Gateway client for interfacing with Switchboard's Crossbar API
pub mod gateway;
pub use gateway::*;

/// Pull-based oracle feed management and data fetching utilities
pub mod pull_feed;
pub use self::pull_feed::PullFeed;

/// SECP256k1 cryptographic utilities and signature verification
pub mod secp256k1;
pub use secp256k1::*;

/// Signature-based authentication for Switchboard services
pub mod signature_auth;
pub use signature_auth::*;

/// Lookup table ownership and management functionality
pub mod lut_owner;
pub use lut_owner::*;

/// Solana slot hash utilities and recent hash management
pub mod recent_slothashes;
pub use recent_slothashes::*;

/// Client-specific account structures and deserialization utilities
pub mod accounts;
pub use accounts::*;

/// Client-specific instruction builders for interacting with the Switchboard On-Demand program
pub mod instructions;
pub use instructions::*;

// Local fork note:
// The upstream client-v3 surface includes modules that currently do not build
// against this workspace's Solana 4.x stack. The CLI only relies on the
// modules above, so we exclude the broken, unused modules here.

/// Re-export prost for protobuf handling
pub use prost;

pub async fn fetch_zerocopy_account<T: bytemuck::Pod + crate::Discriminator + crate::Owner>(
    client: &crate::RpcClient,
    pubkey: crate::Pubkey,
) -> Result<T, crate::OnDemandError> {
    let data = client
        .get_account_data(&pubkey.to_bytes().into())
        .await
        .map_err(|_| crate::OnDemandError::AccountNotFound)?;

    if data.len() < T::discriminator().len() {
        return Err(crate::OnDemandError::InvalidDiscriminator);
    }

    let mut disc_bytes = [0u8; 8];
    disc_bytes.copy_from_slice(&data[..8]);
    if disc_bytes != T::discriminator() {
        return Err(crate::OnDemandError::InvalidDiscriminator);
    }

    Ok(*bytemuck::try_from_bytes::<T>(&data[8..])
        .map_err(|_| crate::OnDemandError::AnchorParseError)?)
}

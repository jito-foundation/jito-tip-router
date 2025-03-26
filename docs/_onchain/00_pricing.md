---
title: Pricing
category: Jekyll
layout: post
---

# Pricing

## Introduction

The Pricing section of the Jito Tip Router defines the mechanisms for managing and updating price data for supported tokens and vaults within the system.
Accurate and efficient pricing is critical for ensuring fair and transparent reward distribution across participants.
This section introduces key components, such as the `VaultRegistry` and `WeightTable`, which are used to store metadata, pricing information, and asset weights.

The pricing architecture leverages oracle feeds, (currently Switchboard), to provide real-time price updates for supported tokens from vaults.
Additionally, it includes fallback mechanisms for scenarios where feeds are unavailable, ensuring continuous operation.
The system supports permissionless interactions for certain roles, such as Crankers, to initialize and update accounts, making the pricing process both robust and decentralized.

This section covers:

- **VaultRegistry**: A registry for storing metadata about vaults and their associated tokens, including pricing feeds and backup weights.
- **WeightTable**: A mechanism to assign weights to assets each epoch based on their relative value, ensuring dynamic and fair pricing adjustments.

The pricing system ensures that all registered vaults and tokens within the Jito Tip Router NCN operate with accurate and transparent price data, fostering trust and efficiency in the ecosystem.


![alt text](/assets/images/pricing.png)
*Figure: Overview of the Pricing


## VaultRegistry

### Initialize VaultRegistry

A Permissionless Cranker initializes the `VaultRegistry` account to store metadata about vaults registered for the Jito Tip Router NCN and inoformation about underlying tokens.
While the [Jito Vault Program] stores all on-chain vault information, the Permissionless Cranker manages key details, quotes important data, and uploads it to the `VaultRegistry`.

```rust
pub struct VaultRegistry {
    ...

    /// The list of supported token ( ST ) mints
    pub st_mint_list: [StMintEntry; 64],

    /// The list of vaults
    pub vault_list: [VaultEntry; 64],
}
```

[Jito Vault Program]: https://docs.restaking.jito.network/vault/00_vault_accounts/

### Register Supported Token Mint (st_mint_list)

The Admin registers a supported token mint using the `process_admin_register_st_mint` instruction.
This stores relevant information in the `st_mint_list` field of the `VaultRegistry`.

Details of each supported token mint include:

- **Token Mint Pubkey**: The unique identifier for the token mint.
- **Pricing Information**: A custom feed or fixed price if no feed is available.

```rust
pub struct StMintEntry {
    /// The supported token ( ST ) mint
    st_mint: Pubkey,

    /// The fee group for the mint
    ncn_fee_group: NcnFeeGroup,

    /// The reward multiplier in basis points
    reward_multiplier_bps: PodU64,

    /// Either a switchboard feed or a no feed weight must be set
    /// The switchboard feed for the mint
    switchboard_feed: Pubkey,

    /// The weight when no feed is available
    no_feed_weight: PodU128,
}
```

This field enables the storage of an oracle feed for each underlying asset (supported token or ST) along with a backup price. Initially, the mints permitted for vaults include **LSTs** and **JTO**. Prices will be quoted in SOL. 

### Register Vault (vault_list)

Permissionless Cranker can register the vault which is associated with Jito Tip Router NCN.
Before running `process_register_vault` instruction, both `NcnVaultTicket` and `VaultNcnTicket` accounts must be activated 

```rust
pub struct VaultEntry {
    /// The vault account
    vault: Pubkey,

    /// The supported token ( ST ) mint of the vault
    st_mint: Pubkey,

    /// The index of the vault in respect to the NCN account
    vault_index: PodU64,

    /// The slot the vault was registered
    slot_registered: PodU64,
}
```


## WeightTable

### Initialize Weight Table

Permissionless Cranker initializes `WeightTable` account each epoch to store weights for each asset.

```rust
pub struct WeightTable {
    ...

    /// The weight table
    table: [WeightEntry; 64],
}
```

### Set weight by Switchboard

The weights will be set for each asset via Switchboard feeds through `switchboard_set_weight` instruction.

```rust
pub struct WeightEntry {
    /// Info about the ST mint
    st_mint_entry: StMintEntry,

    /// The weight of the ST mint
    weight: PodU128,

    /// The slot the weight was set
    slot_set: PodU64,

    /// The slot the weight was last updated
    slot_updated: PodU64,
}
```


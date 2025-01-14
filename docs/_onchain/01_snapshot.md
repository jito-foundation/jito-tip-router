---
title: Snapshot
category: Jekyll
layout: post
---

# Snapshot

In order to have fixed voting weights for consensus, a snapshot process is done to track the stake balances of all operators, and the composition of assets for each operator for reward payouts later in the epoch.
Fees are also stored. This is performed in a top-down approach:

1. Initialize `WeightTable`, finalize it.
2. Initialize `EpochSnapshot`.
3. Initialize `OperatorSnapshot`.
4. Tracks stake weights and stake reward multipliers based on asset type.

## EpochSnapshot

### Initialize & Realloc EpochSnapshot

A Permissionless Cranker initialize and realloc `EpochSnapshot` account to store snapshot which is summary of current epoch.

- `fees`: Snapshot of the NCN Fees for the epoch 
- `operator_count`: The count of operator is associated with Jito Tip Router
- `vault_count`: The count of vault is associated with Jito Tip Router (* number of `VaultEntry` stored in `WeightTable` account)
- `stake_weights`: The total stake weights for all vault operator delegations

```rust
pub struct EpochSnapshot {
    ...

    /// Snapshot of the Fees for the epoch
    fees: Fees,

    /// Number of operators in the epoch
    operator_count: PodU64,

    /// Number of vaults in the epoch
    vault_count: PodU64,

    ...

    /// Tallies the total stake weights for all vault operator delegations
    stake_weights: StakeWeights,
}
```

## OperatorSnapshot

### Initialize & Realloc OperatorSnapshot  

A Permissionless Cranker initialize and realloc `OperatorSnapshot` account each epoch.

```rust
pub struct OperatorSnapshot {
    ...

    operator: Pubkey,

    is_active: PodBool,

    ncn_operator_index: PodU64,
    operator_index: PodU64,
    operator_fee_bps: PodU16,

    vault_operator_delegation_count: PodU64,
    vault_operator_delegations_registered: PodU64,
    valid_operator_vault_delegations: PodU64,

    vault_operator_stake_weight: [VaultOperatorStakeWeight; 64],
}
```


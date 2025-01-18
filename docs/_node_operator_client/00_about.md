---
title: About
category: Jekyll
layout: post
---

# Node Operator Client

## Solana Epoch Snapshot

Once an epoch completes, the operator client takes a snapshot of the Bank from the last existing block of the previous epoch.

## Generate Tip Distribution Merkle Trees and Meta Merkle Root

Using the Bank, the client constructs the StakeMetaCollection, which includes details about each validator, their tip distribution account, and the active delegations (stake accounts) on that validator.

Based on the StakeMetaCollection, the client generates Merkle trees for each validator, resulting in the GeneratedMerkleTreeCollection. Each tree contains a set of "claimants," which include all the validator's stake accounts, an additional claimant representing the validator’s share of tips (calculated using their mev_commission_bps from the tip distribution account), and a claimant for the fee allocated to the TipRouter.

**Details about the `TipRouter` claimant**:

- The inclusion of this claimant is the primary difference between the current creation process for `StakeMetaCollection` and `GeneratedMerkleTreeCollection` compared to the prior approach in [`jito-solana/tip-distributor`].
- The amount for the `TipRouter` claimant is calculated by multiplying the validator's total tips by the `TipRouter`'s `NcnConfig.fees_config.total_fees_bps()`, which aggregates the BaseFees and NcnFees.
- The `TipRouter` fee represents a percentage of fees assessed on each validator's tips. The program only holds the complete fee once all validators’ `TipRouter` claimants have been claimed.
- The fee's destination is a dedicated Program Derived Address (PDA) for each epoch, known as the `BaseRewardReceiver`.

Finally, a MetaMerkleTree is created from the `GeneratedMerkleTreeCollection`.

[`jito-solana/tip-distributor`]: https://github.com/jito-foundation/jito-programs/tree/master/mev-programs/

## Cast Vote

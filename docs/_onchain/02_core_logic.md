---
title: Core Logic
category: Jekyll
layout: post
---

# Core Logic (Cast Votes + Consensus)

## Introduction

The Core Logic section of the Jito Tip Router outlines the mechanisms behind the voting and consensus process, which ensures fair and decentralized decision-making.
By utilizing a structured voting system and consensus rules, the Jito Tip Router enables operators to securely cast votes and reach agreement on key decisions, such as the selection of a meta merkle root.

This section focuses on two main components:

1. **Ballot Box**: A critical account that tracks operator votes, ballot tallies, and consensus progress for each epoch.
2. **Voting and Consensus Mechanism**: The process of casting votes, updating tallies, and reaching a 2/3 consensus based on stake weights.

Once consensus is achieved, the `meta_merkle_root` is finalized, and the system facilitates the seamless integration of this result into downstream processes, such as setting the merkle root for validator Tip Distribution Accounts which enables stakers to claim rewards.

![alt text](/assets/images/core_logic.png)
*Figure: Overview of the Core Logic


## Ballot Box

### Initialize & Realloc BallotBox

A Permissionless Cranker initializes and realloc `BallotBox` account each epoch.

```rust
pub struct BallotBox {
    ...

    /// Slot when this ballot box was created
    slot_created: PodU64,

    /// Slot when consensus was reached
    slot_consensus_reached: PodU64,

    /// Reserved space
    reserved: [u8; 128],

    /// Number of operators that have voted
    operators_voted: PodU64,

    /// Number of unique ballots
    unique_ballots: PodU64,

    /// The ballot that got at least 66% of votes
    winning_ballot: Ballot,

    /// Operator votes
    operator_votes: [OperatorVote; 256],

    /// Mapping of ballots votes to stake weight
    ballot_tallies: [BallotTally; 256],
}
```

## Cast Vote

Operators determine their 32-byte meta_merkle_root off-chain.
This represents a compressed version of the distribution of all tips to all stakers for the target epoch.
They call `cast_vote` instruction with this root, and it is deposited as a `Ballot` into the `BallotBox` account, assuming we are within the valid window of voting.
Tallies are stored for each ballot and continuously updated as votes come in, automatically setting the winning ballot once consensus is reached.

```rust
pub struct Ballot {
    /// The merkle root of the meta merkle tree
    meta_merkle_root: [u8; 32],

    /// Whether the ballot is initialized
    is_initialized: PodBool,
}

pub struct BallotTally {
    ...

    /// The ballot being tallied
    ballot: Ballot,

    /// Breakdown of all of the stake weights that contribute to the vote
    stake_weights: StakeWeights,

    /// The number of votes for this ballot
    tally: PodU64,
}
```

Consensus is defined as 2/3 or greater of the total available stake weight voting for the same meta_merkle_root.

Voting is valid as long as: 
- consensus is not reached.
- consensus is reached and we are not more than config.valid_slots_after_voting slots since consensus was first reached.

Validators can change their votes up until consensus is reached.

## Set Merkle Root

Once a meta merkle root is decided, meaning consensus is reached, each validator’s `TipDistributionAccount` with the merkle_root_upload_authority set to the NcnConfig can have its own merkle_root set.
The Cranker client will invoke SetMerkleRoot with the merkle proof, and all the arguments for the Tip Distribution Program UploadMerkleRoot instruction for a given validator.
These arguments make up the leaf node of the tree, so the proof is verified against the meta_merkle_root, and a CPI sets the merkle root on the TipDistributionAccount.
Claims for that validator and its stakers can now begin.



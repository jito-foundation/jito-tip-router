---
title: Overview
category: Jekyll
layout: post
---

Jito Tip Router NCN is handling operation of distribution of MEV tips generated from the Jito Tip Distribution protocol. The system is made of 3 components: 

- Onchain NCN program
- Node Operator Client
- Permissionless Cranker

#### Onchain NCN Program (Jito Tip Router Program):

Onchain NCN program has several components:

- Pricing
  - Switchboard determines the relative weight of assets ( jitoSOL, JTO, ... ) deposited in all the Vaults linked to this Jito Tip Router NCN.

- Snapshot
  - Take snapshots of Operator and Vault per epoch.

- Core Logic (Consensus)
  - Prepare Ballot Box, all votes would be collected here. 
  - Each operator calculate the merkle tree to produce merkle root then cast vote with produced merkle root.
  - After consensus reached with more than 2/3, cranker can upload the merkle tree of each validator.

- Reward Payment

#### Node Operator Client

- Node operators will compute a `meta merkle root` — a merkle root derived from a new merkle tree containing all validator merkle trees.
- Upload `meta merkle root` on-chain.


#### Permissionless Cranker

- Take snapshots of Operator and Vault per epoch.


![alt text](/assets/images/overview.png)
*Figure: Overview of the Jito Tip Router*


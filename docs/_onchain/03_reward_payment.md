---
title: Reward Payment
category: Jekyll
layout: post
---

# Reward Payment

## Introduction

The Reward Payment module in the Jito Tip Router is responsible for distributing rewards generated from tips.
It ensures efficient routing and allocation of rewards to all relevant parties, including base reward recipients, operators, and vaults.
This section details the routing and distribution process, critical instructions, and key components involved in the reward payment workflow.

![alt text](/assets/images/reward_payment.png)
*Figure: Overview of the Reward Payment

## Routing and Distribution Process

1. Rewards ( in lamports ) are sent to the PDA of the `BaseRewardReceiver`.
2. The `route_base_rewards` instruction is caleed *x* times until `still_routing` becomes `false`. (This is typically only once but may require multiple calls at higher levels of operators and vaults within the network due to CU limitations).
3. Once routing is complete, rewards can be distributed:
    a. Use `distribute_base_rewards` instruction to allocate to the base reward recipients. (in JitoSOL).
    b. Use `distribute_ncn_operator_rewards` to send rewards to the next router, specifically the `NcnRewardReceiver` (in lamports), which corresponds to one per operator per NCN fee group.
4. The `route_ncn_rewards` instruction is called *x* times until `still_routing` becomes `false`
5. Once routing is complete, rewards can be distributed: 
    a. Use `distribute_ncn_operator_rewards` to allocate rewards to the operators (in JitoSOL).
    b. Use `distribute_ncn_vault_rewards` to allocate rewards to the vault (in JitoSOL).

This system enables reward distribution (in lamports) at any time after consensus is achieved, regardless of the amount.

The most critical instructions in this process are `route_base_rewards` and `route_ncn_rewards`, with particular emphasis on the calculation functions they invoke.
It is important to highlight that the router does not consider the specific percentages allocated to each party but rather focuses on their ratios to determine the distribution proportions.


## Key Components

### BaseRewardRouter

1. Core Purpose

The `BaseRewardRouter` is designed to:

- **Manage Rewards**: Keep track of rewards to be distributed across different groups and operators.
- **Route Rewards**: Handle the allocation and routing of rewards from a reward pool to various fee groups and operators.
- **Support State Persistence**: Save and resume the state of routing operations to handle large computaions and ensure continuity.

2. Key Concepts

- **Base and NCN Fee Groups**:
    - Rewards are divided into base fee groups (e.g., protocol and DAO fees) and NCN fee groups (e.g., operator-specific fees).
    - Each group has specific routing and distribution logic.

- **Routing and Distribution**:
    - **Routing**: Calculates and assigns rewards to the correct pools or routes.
    - **Distribution**: Transfers rewards from the router to recipients (e.g., operators or vaults).

- **Persistence and State Management**:
    - Supports resuming routing from a saved state to handle large-scale operations within computational limits.

### NcnRewardRouter

1. Core Purpose

The NcnRewardRoute is designed to:

- Track Operator Rewards: Maintain a record of rewards assigned to an operator across all NCN fee groups.
- Enable Reward Updates: Allow incrementing or decrementing rewards based on operations or distributions.
- Support Validation and Checks: Provide utility functions to check reward states (e.g., if rewards exist).


---
title: Reward Payment
category: Jekyll
layout: post
---

# Reward Payment

The routing and distribution process proceeds as follows:

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


## BaseRewardRouter

## NcnRewardRouter

## BaseRewards

## NcnBaseRewards

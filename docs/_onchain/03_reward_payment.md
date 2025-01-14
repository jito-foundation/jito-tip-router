---
title: Reward Payment
category: Jekyll
layout: post
---

# Reward Payment

The routing and distributing flow goes as follows:

1. Rewards ( in lamports ) are sent to the `BaseRewardReceiver`’s PDA.
2. `route_base_rewards` is caleed *x* times until `still_routing` is `false`. (probably only one time, but we run into a CU issue at high level of operators and vaults within the network.)
3. Now rewards can be distributed:
    a. Call `distribute_base_rewards` to distribute the base reward recipients. (in JitoSOL).
    b. Call `distribute_ncn_operator_rewards` to distribute to the next router, specifically the `NcnRewardReceiver` (in lamports - which there is one per operator per NCN fee group.
4. `route_ncn_rewards` is called *x* times until `still_routing` is `false`
5. Now rewards can be distributed:
    a. Call `distribute_ncn_operator_rewards` to distribute to the operator. (in JitoSOL)
    b. Call `distribute_ncn_vault_rewards` to distribute to the vault. (in JitoSOL)

This system allows distribution of rewards ( lamports ) at anytime after consensus has been reached, for any amount. 

The most crucial instructions here are route_base_rewards and route_ncn_rewards, more specifically, the calculation functions within these.
It is important to note that the router does not care what percentages each party takes, rather the ratio of those percentages to determine who gets what.


## BaseRewardRouter

## NcnRewardRouter

## BaseRewards

## NcnBaseRewards

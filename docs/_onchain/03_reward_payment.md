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

## Reward Payment Workflow Overview

1. Rewards ( in lamports ) are sent to the PDA of the `BaseRewardReceiver` (Permissionless cranker will claim the rewards).
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


## Step-by-Step Reward Payment Instructions

### 1. Route Base Rewards

It handles routing rewards from the `BaseRewardReceiver` to the `BaseRewardRouter` and further processes the allocation of base rewards (DAO, ...) and NCN fee group rewards (Operator).

![alt text](/assets/images/route_base_rewards.png)
*Figure: Overview of the Route Base Rewards

```rust
pub fn route_reward_pool(&mut self, fee: &Fees) -> Result<(), TipRouterError> {
    let rewards_to_process: u64 = self.reward_pool();

    let total_fee_bps = fee.total_fees_bps()?;

    // Base Fee Group Rewards
    for group in BaseFeeGroup::all_groups().iter() {
        let base_fee = fee.base_fee_bps(*group)?;

        let rewards =
            Self::calculate_reward_split(base_fee, total_fee_bps, rewards_to_process)?;

        self.route_from_reward_pool(rewards)?;
        self.route_to_base_fee_group_rewards(*group, rewards)?;
    }

    // NCN Fee Group Rewards
    for group in NcnFeeGroup::all_groups().iter() {
        let ncn_group_fee = fee.ncn_fee_bps(*group)?;

        let rewards =
            Self::calculate_reward_split(ncn_group_fee, total_fee_bps, rewards_to_process)?;

        self.route_from_reward_pool(rewards)?;
        self.route_to_ncn_fee_group_rewards(*group, rewards)?;
    }

    // DAO gets any remainder
    {
        let leftover_rewards = self.reward_pool();

        self.route_from_reward_pool(leftover_rewards)?;
        self.route_to_base_fee_group_rewards(BaseFeeGroup::dao(), leftover_rewards)?;
    }

    Ok(())
}

```

### 2. Distribute Base Rewards

This ensures that all base reward recipients, such as DAO, receive their appropriate share of the rewards generated during the epoch.
This instruction integrates with the Solana Stake Pool program to deposit rewards in JitoSOL and utilizes both on-chain accounts and external token accounts to manage distribution efficiently.

![alt text](/assets/images/distribute_base_rewards.png)
*Figure: Overview of the Distribute Base Rewards

```rust
let deposit_ix = deposit_sol(
    stake_pool_program.key,
    stake_pool.key,
    stake_pool_withdraw_authority.key,
    reserve_stake.key,
    base_reward_receiver.key,
    base_fee_wallet_ata.key,
    manager_fee_account.key,
    referrer_pool_tokens_account.key,
    pool_mint.key,
    token_program.key,
    rewards,
);

// Invoke the deposit instruction with base_reward_router as signer
invoke_signed(
    &deposit_ix,
    &[
        stake_pool.clone(),
        stake_pool_withdraw_authority.clone(),
        reserve_stake.clone(),
        base_reward_receiver.clone(),
        base_fee_wallet_ata.clone(),
        manager_fee_account.clone(),
        referrer_pool_tokens_account.clone(),
        pool_mint.clone(),
        system_program.clone(),
        token_program.clone(),
    ],
    &[base_reward_receiver_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect::<Vec<&[u8]>>()
        .as_slice()],
)?;
```

### 3. Distribute NCN Reward Route

It handles the distribution of rewards from the `BaseRewardReceiver` to the `NcnRewardReceiver` for a specific NCN fee group and operator.
This instruction ensures that rewards are routed accurately to operators within the NCN fee groups (Operator), based on their contributions and stake weights.

![alt text](/assets/images/distribute_base_ncn_reward_route.png)
*Figure: Overview of the Distribute Base NCN Reward Route

```rust
// Get rewards and update state
let rewards = {
    let mut epoch_reward_router_data = base_reward_router.try_borrow_mut_data()?;
    let base_reward_router_account =
        BaseRewardRouter::try_from_slice_unchecked_mut(&mut epoch_reward_router_data)?;

    if base_reward_router_account.still_routing() {
        msg!("Rewards still routing");
        return Err(TipRouterError::RouterStillRouting.into());
    }

    base_reward_router_account
        .distribute_ncn_fee_group_reward_route(ncn_fee_group, operator.key)?
};

// Send rewards
if rewards > 0 {
    let (_, base_reward_receiver_bump, mut base_reward_receiver_seeds) =
        BaseRewardReceiver::find_program_address(program_id, ncn.key, epoch);
    base_reward_receiver_seeds.push(vec![base_reward_receiver_bump]);

    solana_program::program::invoke_signed(
        &solana_program::system_instruction::transfer(
            base_reward_receiver.key,
            ncn_reward_receiver.key,
            rewards,
        ),
        &[base_reward_receiver.clone(), ncn_reward_receiver.clone()],
        &[base_reward_receiver_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()
            .as_slice()],
    )?;
}
```

### 4. Route NCN Reward

Its primary function is to calculate and prepare reward allocations for operators and fee groups, without actually transferring rewards.
This instruction is focused on determining how rewards should be distributed by processing operator snapshots, fee group configurations, and available rewards within the system.

![alt text](/assets/images/route_ncn_rewards.png)
*Figure: Overview of the Route NCN Rewards

```rust
...

    for vault_operator_delegation_index in starting_vault_operator_delegation_index
        ..operator_snapshot.vault_operator_stake_weight().len()
    {
        let vault_operator_delegation = operator_snapshot.vault_operator_stake_weight()
            [vault_operator_delegation_index];

        // Update iteration state
        {
            iterations = iterations
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?;

            if iterations > max_iterations {
                msg!(
                    "Reached max iterations, saving state and exiting {}/{}",
                    rewards_to_process,
                    vault_operator_delegation_index
                );
                self.save_routing_state(
                    rewards_to_process,
                    vault_operator_delegation_index,
                );
                return Ok(());
            }
        }

        let vault = vault_operator_delegation.vault();

        let vault_reward_stake_weight = vault_operator_delegation
            .stake_weights()
            .ncn_fee_group_stake_weight(vault_ncn_fee_group)?;

        let operator_reward_stake_weight =
            operator_stake_weight.ncn_fee_group_stake_weight(vault_ncn_fee_group)?;

        let vault_reward = Self::calculate_vault_reward(
            vault_reward_stake_weight,
            operator_reward_stake_weight,
            rewards_to_process,
        )?;

        self.route_from_reward_pool(vault_reward)?;
        self.route_to_vault_reward_route(vault, vault_reward)?;
    }

    self.reset_routing_state();
}

// Operator gets any remainder
{
    let leftover_rewards = self.reward_pool();

    self.route_from_reward_pool(leftover_rewards)?;
    self.route_to_operator_rewards(leftover_rewards)?;
}
```

### 5. Distribute NCN Operator Rewards

This instruction ensures that the calculated rewards for each operator within a specific NCN fee group are distributed appropriately.
It moves rewards from the NcnRewardReceiver to the operator's associated token account, converting them into a JitoSOL.

![alt text](/assets/images/distribute_ncn_operator_rewards.png)
*Figure: Overview of the Distribute NCN Operator Rewards

```rust
let (_, ncn_reward_receiver_bump, mut ncn_reward_receiver_seeds) =
    NcnRewardReceiver::find_program_address(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
    );
ncn_reward_receiver_seeds.push(vec![ncn_reward_receiver_bump]);

let deposit_ix = deposit_sol(
    stake_pool_program.key,
    stake_pool.key,
    stake_pool_withdraw_authority.key,
    reserve_stake.key,
    ncn_reward_receiver.key,
    operator_ata.key,
    manager_fee_account.key,
    referrer_pool_tokens_account.key,
    pool_mint.key,
    token_program.key,
    rewards,
);

// Invoke the deposit instruction with ncn_reward_receiver as signer
invoke_signed(
    &deposit_ix,
    &[
        stake_pool.clone(),
        stake_pool_withdraw_authority.clone(),
        reserve_stake.clone(),
        ncn_reward_receiver.clone(),
        operator_ata.clone(),
        manager_fee_account.clone(),
        referrer_pool_tokens_account.clone(),
        pool_mint.clone(),
        system_program.clone(),
        token_program.clone(),
    ],
    &[ncn_reward_receiver_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect::<Vec<&[u8]>>()
        .as_slice()],
)?;
```

### 6. Distribute NCN Vault Rewards

This instruction calculates the rewards for a vault within a particular NCN fee group and operator, transfers the rewards, and integrates them into the stake pool system (e.g., depositing them as JitoSOL).

![alt text](/assets/images/distribute_ncn_vault_rewards.png)
*Figure: Overview of the Distribute NCN Vault Rewards

```rust
let (_, ncn_reward_receiver_bump, mut ncn_reward_receiver_seeds) =
    NcnRewardReceiver::find_program_address(
        program_id,
        ncn_fee_group,
        operator.key,
        ncn.key,
        epoch,
    );
ncn_reward_receiver_seeds.push(vec![ncn_reward_receiver_bump]);

let deposit_ix = deposit_sol(
    stake_pool_program.key,
    stake_pool.key,
    stake_pool_withdraw_authority.key,
    reserve_stake.key,
    ncn_reward_receiver.key,
    vault_ata.key,
    manager_fee_account.key,
    referrer_pool_tokens_account.key,
    pool_mint.key,
    token_program.key,
    rewards,
);

// Invoke the deposit instruction with ncn_reward_receiver as signer
invoke_signed(
    &deposit_ix,
    &[
        stake_pool.clone(),
        stake_pool_withdraw_authority.clone(),
        reserve_stake.clone(),
        ncn_reward_receiver.clone(),
        vault_ata.clone(),
        manager_fee_account.clone(),
        referrer_pool_tokens_account.clone(),
        pool_mint.clone(),
        system_program.clone(),
        token_program.clone(),
        stake_pool_program.clone(),
    ],
    &[ncn_reward_receiver_seeds
        .iter()
        .map(|s| s.as_slice())
        .collect::<Vec<&[u8]>>()
        .as_slice()],
)?;
```


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


//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};

/// Accounts.
pub struct DistributeNcnOperatorRewards {
    pub epoch_state: solana_program::pubkey::Pubkey,

    pub config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub operator: solana_program::pubkey::Pubkey,

    pub operator_ata: solana_program::pubkey::Pubkey,

    pub operator_snapshot: solana_program::pubkey::Pubkey,

    pub ncn_reward_router: solana_program::pubkey::Pubkey,

    pub ncn_reward_receiver: solana_program::pubkey::Pubkey,

    pub restaking_program: solana_program::pubkey::Pubkey,

    pub stake_pool_program: solana_program::pubkey::Pubkey,

    pub stake_pool: solana_program::pubkey::Pubkey,

    pub stake_pool_withdraw_authority: solana_program::pubkey::Pubkey,

    pub reserve_stake: solana_program::pubkey::Pubkey,

    pub manager_fee_account: solana_program::pubkey::Pubkey,

    pub referrer_pool_tokens_account: solana_program::pubkey::Pubkey,

    pub pool_mint: solana_program::pubkey::Pubkey,

    pub token_program: solana_program::pubkey::Pubkey,

    pub system_program: solana_program::pubkey::Pubkey,
}

impl DistributeNcnOperatorRewards {
    pub fn instruction(
        &self,
        args: DistributeNcnOperatorRewardsInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: DistributeNcnOperatorRewardsInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(18 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.epoch_state,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.operator,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.operator_ata,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.operator_snapshot,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.ncn_reward_router,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.ncn_reward_receiver,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.stake_pool_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.stake_pool,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.stake_pool_withdraw_authority,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.reserve_stake,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.manager_fee_account,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.referrer_pool_tokens_account,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.pool_mint,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.token_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.system_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = DistributeNcnOperatorRewardsInstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = args.try_to_vec().unwrap();
        data.append(&mut args);

        solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DistributeNcnOperatorRewardsInstructionData {
    discriminator: u8,
}

impl DistributeNcnOperatorRewardsInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 24 }
    }
}

impl Default for DistributeNcnOperatorRewardsInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DistributeNcnOperatorRewardsInstructionArgs {
    pub ncn_fee_group: u8,
    pub epoch: u64,
}

/// Instruction builder for `DistributeNcnOperatorRewards`.
///
/// ### Accounts:
///
///   0. `[writable]` epoch_state
///   1. `[]` config
///   2. `[]` ncn
///   3. `[writable]` operator
///   4. `[writable]` operator_ata
///   5. `[writable]` operator_snapshot
///   6. `[writable]` ncn_reward_router
///   7. `[writable]` ncn_reward_receiver
///   8. `[]` restaking_program
///   9. `[]` stake_pool_program
///   10. `[writable]` stake_pool
///   11. `[]` stake_pool_withdraw_authority
///   12. `[writable]` reserve_stake
///   13. `[writable]` manager_fee_account
///   14. `[writable]` referrer_pool_tokens_account
///   15. `[writable]` pool_mint
///   16. `[optional]` token_program (default to `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`)
///   17. `[optional]` system_program (default to `11111111111111111111111111111111`)
#[derive(Clone, Debug, Default)]
pub struct DistributeNcnOperatorRewardsBuilder {
    epoch_state: Option<solana_program::pubkey::Pubkey>,
    config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    operator: Option<solana_program::pubkey::Pubkey>,
    operator_ata: Option<solana_program::pubkey::Pubkey>,
    operator_snapshot: Option<solana_program::pubkey::Pubkey>,
    ncn_reward_router: Option<solana_program::pubkey::Pubkey>,
    ncn_reward_receiver: Option<solana_program::pubkey::Pubkey>,
    restaking_program: Option<solana_program::pubkey::Pubkey>,
    stake_pool_program: Option<solana_program::pubkey::Pubkey>,
    stake_pool: Option<solana_program::pubkey::Pubkey>,
    stake_pool_withdraw_authority: Option<solana_program::pubkey::Pubkey>,
    reserve_stake: Option<solana_program::pubkey::Pubkey>,
    manager_fee_account: Option<solana_program::pubkey::Pubkey>,
    referrer_pool_tokens_account: Option<solana_program::pubkey::Pubkey>,
    pool_mint: Option<solana_program::pubkey::Pubkey>,
    token_program: Option<solana_program::pubkey::Pubkey>,
    system_program: Option<solana_program::pubkey::Pubkey>,
    ncn_fee_group: Option<u8>,
    epoch: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl DistributeNcnOperatorRewardsBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn epoch_state(&mut self, epoch_state: solana_program::pubkey::Pubkey) -> &mut Self {
        self.epoch_state = Some(epoch_state);
        self
    }
    #[inline(always)]
    pub fn config(&mut self, config: solana_program::pubkey::Pubkey) -> &mut Self {
        self.config = Some(config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn operator(&mut self, operator: solana_program::pubkey::Pubkey) -> &mut Self {
        self.operator = Some(operator);
        self
    }
    #[inline(always)]
    pub fn operator_ata(&mut self, operator_ata: solana_program::pubkey::Pubkey) -> &mut Self {
        self.operator_ata = Some(operator_ata);
        self
    }
    #[inline(always)]
    pub fn operator_snapshot(
        &mut self,
        operator_snapshot: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.operator_snapshot = Some(operator_snapshot);
        self
    }
    #[inline(always)]
    pub fn ncn_reward_router(
        &mut self,
        ncn_reward_router: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.ncn_reward_router = Some(ncn_reward_router);
        self
    }
    #[inline(always)]
    pub fn ncn_reward_receiver(
        &mut self,
        ncn_reward_receiver: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.ncn_reward_receiver = Some(ncn_reward_receiver);
        self
    }
    #[inline(always)]
    pub fn restaking_program(
        &mut self,
        restaking_program: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.restaking_program = Some(restaking_program);
        self
    }
    #[inline(always)]
    pub fn stake_pool_program(
        &mut self,
        stake_pool_program: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.stake_pool_program = Some(stake_pool_program);
        self
    }
    #[inline(always)]
    pub fn stake_pool(&mut self, stake_pool: solana_program::pubkey::Pubkey) -> &mut Self {
        self.stake_pool = Some(stake_pool);
        self
    }
    #[inline(always)]
    pub fn stake_pool_withdraw_authority(
        &mut self,
        stake_pool_withdraw_authority: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.stake_pool_withdraw_authority = Some(stake_pool_withdraw_authority);
        self
    }
    #[inline(always)]
    pub fn reserve_stake(&mut self, reserve_stake: solana_program::pubkey::Pubkey) -> &mut Self {
        self.reserve_stake = Some(reserve_stake);
        self
    }
    #[inline(always)]
    pub fn manager_fee_account(
        &mut self,
        manager_fee_account: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.manager_fee_account = Some(manager_fee_account);
        self
    }
    #[inline(always)]
    pub fn referrer_pool_tokens_account(
        &mut self,
        referrer_pool_tokens_account: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.referrer_pool_tokens_account = Some(referrer_pool_tokens_account);
        self
    }
    #[inline(always)]
    pub fn pool_mint(&mut self, pool_mint: solana_program::pubkey::Pubkey) -> &mut Self {
        self.pool_mint = Some(pool_mint);
        self
    }
    /// `[optional account, default to 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA']`
    #[inline(always)]
    pub fn token_program(&mut self, token_program: solana_program::pubkey::Pubkey) -> &mut Self {
        self.token_program = Some(token_program);
        self
    }
    /// `[optional account, default to '11111111111111111111111111111111']`
    #[inline(always)]
    pub fn system_program(&mut self, system_program: solana_program::pubkey::Pubkey) -> &mut Self {
        self.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn ncn_fee_group(&mut self, ncn_fee_group: u8) -> &mut Self {
        self.ncn_fee_group = Some(ncn_fee_group);
        self
    }
    #[inline(always)]
    pub fn epoch(&mut self, epoch: u64) -> &mut Self {
        self.epoch = Some(epoch);
        self
    }
    /// Add an additional account to the instruction.
    #[inline(always)]
    pub fn add_remaining_account(
        &mut self,
        account: solana_program::instruction::AccountMeta,
    ) -> &mut Self {
        self.__remaining_accounts.push(account);
        self
    }
    /// Add additional accounts to the instruction.
    #[inline(always)]
    pub fn add_remaining_accounts(
        &mut self,
        accounts: &[solana_program::instruction::AccountMeta],
    ) -> &mut Self {
        self.__remaining_accounts.extend_from_slice(accounts);
        self
    }
    #[allow(clippy::clone_on_copy)]
    pub fn instruction(&self) -> solana_program::instruction::Instruction {
        let accounts = DistributeNcnOperatorRewards {
            epoch_state: self.epoch_state.expect("epoch_state is not set"),
            config: self.config.expect("config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            operator: self.operator.expect("operator is not set"),
            operator_ata: self.operator_ata.expect("operator_ata is not set"),
            operator_snapshot: self
                .operator_snapshot
                .expect("operator_snapshot is not set"),
            ncn_reward_router: self
                .ncn_reward_router
                .expect("ncn_reward_router is not set"),
            ncn_reward_receiver: self
                .ncn_reward_receiver
                .expect("ncn_reward_receiver is not set"),
            restaking_program: self
                .restaking_program
                .expect("restaking_program is not set"),
            stake_pool_program: self
                .stake_pool_program
                .expect("stake_pool_program is not set"),
            stake_pool: self.stake_pool.expect("stake_pool is not set"),
            stake_pool_withdraw_authority: self
                .stake_pool_withdraw_authority
                .expect("stake_pool_withdraw_authority is not set"),
            reserve_stake: self.reserve_stake.expect("reserve_stake is not set"),
            manager_fee_account: self
                .manager_fee_account
                .expect("manager_fee_account is not set"),
            referrer_pool_tokens_account: self
                .referrer_pool_tokens_account
                .expect("referrer_pool_tokens_account is not set"),
            pool_mint: self.pool_mint.expect("pool_mint is not set"),
            token_program: self.token_program.unwrap_or(solana_program::pubkey!(
                "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            )),
            system_program: self
                .system_program
                .unwrap_or(solana_program::pubkey!("11111111111111111111111111111111")),
        };
        let args = DistributeNcnOperatorRewardsInstructionArgs {
            ncn_fee_group: self
                .ncn_fee_group
                .clone()
                .expect("ncn_fee_group is not set"),
            epoch: self.epoch.clone().expect("epoch is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `distribute_ncn_operator_rewards` CPI accounts.
pub struct DistributeNcnOperatorRewardsCpiAccounts<'a, 'b> {
    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator_ata: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator_snapshot: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_reward_router: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool_withdraw_authority: &'b solana_program::account_info::AccountInfo<'a>,

    pub reserve_stake: &'b solana_program::account_info::AccountInfo<'a>,

    pub manager_fee_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub referrer_pool_tokens_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub pool_mint: &'b solana_program::account_info::AccountInfo<'a>,

    pub token_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `distribute_ncn_operator_rewards` CPI instruction.
pub struct DistributeNcnOperatorRewardsCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator_ata: &'b solana_program::account_info::AccountInfo<'a>,

    pub operator_snapshot: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_reward_router: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool: &'b solana_program::account_info::AccountInfo<'a>,

    pub stake_pool_withdraw_authority: &'b solana_program::account_info::AccountInfo<'a>,

    pub reserve_stake: &'b solana_program::account_info::AccountInfo<'a>,

    pub manager_fee_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub referrer_pool_tokens_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub pool_mint: &'b solana_program::account_info::AccountInfo<'a>,

    pub token_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: DistributeNcnOperatorRewardsInstructionArgs,
}

impl<'a, 'b> DistributeNcnOperatorRewardsCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: DistributeNcnOperatorRewardsCpiAccounts<'a, 'b>,
        args: DistributeNcnOperatorRewardsInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            epoch_state: accounts.epoch_state,
            config: accounts.config,
            ncn: accounts.ncn,
            operator: accounts.operator,
            operator_ata: accounts.operator_ata,
            operator_snapshot: accounts.operator_snapshot,
            ncn_reward_router: accounts.ncn_reward_router,
            ncn_reward_receiver: accounts.ncn_reward_receiver,
            restaking_program: accounts.restaking_program,
            stake_pool_program: accounts.stake_pool_program,
            stake_pool: accounts.stake_pool,
            stake_pool_withdraw_authority: accounts.stake_pool_withdraw_authority,
            reserve_stake: accounts.reserve_stake,
            manager_fee_account: accounts.manager_fee_account,
            referrer_pool_tokens_account: accounts.referrer_pool_tokens_account,
            pool_mint: accounts.pool_mint,
            token_program: accounts.token_program,
            system_program: accounts.system_program,
            __args: args,
        }
    }
    #[inline(always)]
    pub fn invoke(&self) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(&[], &[])
    }
    #[inline(always)]
    pub fn invoke_with_remaining_accounts(
        &self,
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(&[], remaining_accounts)
    }
    #[inline(always)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(signers_seeds, &[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed_with_remaining_accounts(
        &self,
        signers_seeds: &[&[&[u8]]],
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        let mut accounts = Vec::with_capacity(18 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.epoch_state.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.operator.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.operator_ata.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.operator_snapshot.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.ncn_reward_router.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.ncn_reward_receiver.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.restaking_program.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.stake_pool_program.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.stake_pool.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.stake_pool_withdraw_authority.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.reserve_stake.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.manager_fee_account.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.referrer_pool_tokens_account.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.pool_mint.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.token_program.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.system_program.key,
            false,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = DistributeNcnOperatorRewardsInstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(18 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.epoch_state.clone());
        account_infos.push(self.config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.operator.clone());
        account_infos.push(self.operator_ata.clone());
        account_infos.push(self.operator_snapshot.clone());
        account_infos.push(self.ncn_reward_router.clone());
        account_infos.push(self.ncn_reward_receiver.clone());
        account_infos.push(self.restaking_program.clone());
        account_infos.push(self.stake_pool_program.clone());
        account_infos.push(self.stake_pool.clone());
        account_infos.push(self.stake_pool_withdraw_authority.clone());
        account_infos.push(self.reserve_stake.clone());
        account_infos.push(self.manager_fee_account.clone());
        account_infos.push(self.referrer_pool_tokens_account.clone());
        account_infos.push(self.pool_mint.clone());
        account_infos.push(self.token_program.clone());
        account_infos.push(self.system_program.clone());
        remaining_accounts
            .iter()
            .for_each(|remaining_account| account_infos.push(remaining_account.0.clone()));

        if signers_seeds.is_empty() {
            solana_program::program::invoke(&instruction, &account_infos)
        } else {
            solana_program::program::invoke_signed(&instruction, &account_infos, signers_seeds)
        }
    }
}

/// Instruction builder for `DistributeNcnOperatorRewards` via CPI.
///
/// ### Accounts:
///
///   0. `[writable]` epoch_state
///   1. `[]` config
///   2. `[]` ncn
///   3. `[writable]` operator
///   4. `[writable]` operator_ata
///   5. `[writable]` operator_snapshot
///   6. `[writable]` ncn_reward_router
///   7. `[writable]` ncn_reward_receiver
///   8. `[]` restaking_program
///   9. `[]` stake_pool_program
///   10. `[writable]` stake_pool
///   11. `[]` stake_pool_withdraw_authority
///   12. `[writable]` reserve_stake
///   13. `[writable]` manager_fee_account
///   14. `[writable]` referrer_pool_tokens_account
///   15. `[writable]` pool_mint
///   16. `[]` token_program
///   17. `[]` system_program
#[derive(Clone, Debug)]
pub struct DistributeNcnOperatorRewardsCpiBuilder<'a, 'b> {
    instruction: Box<DistributeNcnOperatorRewardsCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> DistributeNcnOperatorRewardsCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(DistributeNcnOperatorRewardsCpiBuilderInstruction {
            __program: program,
            epoch_state: None,
            config: None,
            ncn: None,
            operator: None,
            operator_ata: None,
            operator_snapshot: None,
            ncn_reward_router: None,
            ncn_reward_receiver: None,
            restaking_program: None,
            stake_pool_program: None,
            stake_pool: None,
            stake_pool_withdraw_authority: None,
            reserve_stake: None,
            manager_fee_account: None,
            referrer_pool_tokens_account: None,
            pool_mint: None,
            token_program: None,
            system_program: None,
            ncn_fee_group: None,
            epoch: None,
            __remaining_accounts: Vec::new(),
        });
        Self { instruction }
    }
    #[inline(always)]
    pub fn epoch_state(
        &mut self,
        epoch_state: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.epoch_state = Some(epoch_state);
        self
    }
    #[inline(always)]
    pub fn config(
        &mut self,
        config: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.config = Some(config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn operator(
        &mut self,
        operator: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.operator = Some(operator);
        self
    }
    #[inline(always)]
    pub fn operator_ata(
        &mut self,
        operator_ata: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.operator_ata = Some(operator_ata);
        self
    }
    #[inline(always)]
    pub fn operator_snapshot(
        &mut self,
        operator_snapshot: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.operator_snapshot = Some(operator_snapshot);
        self
    }
    #[inline(always)]
    pub fn ncn_reward_router(
        &mut self,
        ncn_reward_router: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ncn_reward_router = Some(ncn_reward_router);
        self
    }
    #[inline(always)]
    pub fn ncn_reward_receiver(
        &mut self,
        ncn_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ncn_reward_receiver = Some(ncn_reward_receiver);
        self
    }
    #[inline(always)]
    pub fn restaking_program(
        &mut self,
        restaking_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.restaking_program = Some(restaking_program);
        self
    }
    #[inline(always)]
    pub fn stake_pool_program(
        &mut self,
        stake_pool_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.stake_pool_program = Some(stake_pool_program);
        self
    }
    #[inline(always)]
    pub fn stake_pool(
        &mut self,
        stake_pool: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.stake_pool = Some(stake_pool);
        self
    }
    #[inline(always)]
    pub fn stake_pool_withdraw_authority(
        &mut self,
        stake_pool_withdraw_authority: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.stake_pool_withdraw_authority = Some(stake_pool_withdraw_authority);
        self
    }
    #[inline(always)]
    pub fn reserve_stake(
        &mut self,
        reserve_stake: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.reserve_stake = Some(reserve_stake);
        self
    }
    #[inline(always)]
    pub fn manager_fee_account(
        &mut self,
        manager_fee_account: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.manager_fee_account = Some(manager_fee_account);
        self
    }
    #[inline(always)]
    pub fn referrer_pool_tokens_account(
        &mut self,
        referrer_pool_tokens_account: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.referrer_pool_tokens_account = Some(referrer_pool_tokens_account);
        self
    }
    #[inline(always)]
    pub fn pool_mint(
        &mut self,
        pool_mint: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.pool_mint = Some(pool_mint);
        self
    }
    #[inline(always)]
    pub fn token_program(
        &mut self,
        token_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.token_program = Some(token_program);
        self
    }
    #[inline(always)]
    pub fn system_program(
        &mut self,
        system_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn ncn_fee_group(&mut self, ncn_fee_group: u8) -> &mut Self {
        self.instruction.ncn_fee_group = Some(ncn_fee_group);
        self
    }
    #[inline(always)]
    pub fn epoch(&mut self, epoch: u64) -> &mut Self {
        self.instruction.epoch = Some(epoch);
        self
    }
    /// Add an additional account to the instruction.
    #[inline(always)]
    pub fn add_remaining_account(
        &mut self,
        account: &'b solana_program::account_info::AccountInfo<'a>,
        is_writable: bool,
        is_signer: bool,
    ) -> &mut Self {
        self.instruction
            .__remaining_accounts
            .push((account, is_writable, is_signer));
        self
    }
    /// Add additional accounts to the instruction.
    ///
    /// Each account is represented by a tuple of the `AccountInfo`, a `bool` indicating whether the account is writable or not,
    /// and a `bool` indicating whether the account is a signer or not.
    #[inline(always)]
    pub fn add_remaining_accounts(
        &mut self,
        accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> &mut Self {
        self.instruction
            .__remaining_accounts
            .extend_from_slice(accounts);
        self
    }
    #[inline(always)]
    pub fn invoke(&self) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed(&[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        let args = DistributeNcnOperatorRewardsInstructionArgs {
            ncn_fee_group: self
                .instruction
                .ncn_fee_group
                .clone()
                .expect("ncn_fee_group is not set"),
            epoch: self.instruction.epoch.clone().expect("epoch is not set"),
        };
        let instruction = DistributeNcnOperatorRewardsCpi {
            __program: self.instruction.__program,

            epoch_state: self
                .instruction
                .epoch_state
                .expect("epoch_state is not set"),

            config: self.instruction.config.expect("config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            operator: self.instruction.operator.expect("operator is not set"),

            operator_ata: self
                .instruction
                .operator_ata
                .expect("operator_ata is not set"),

            operator_snapshot: self
                .instruction
                .operator_snapshot
                .expect("operator_snapshot is not set"),

            ncn_reward_router: self
                .instruction
                .ncn_reward_router
                .expect("ncn_reward_router is not set"),

            ncn_reward_receiver: self
                .instruction
                .ncn_reward_receiver
                .expect("ncn_reward_receiver is not set"),

            restaking_program: self
                .instruction
                .restaking_program
                .expect("restaking_program is not set"),

            stake_pool_program: self
                .instruction
                .stake_pool_program
                .expect("stake_pool_program is not set"),

            stake_pool: self.instruction.stake_pool.expect("stake_pool is not set"),

            stake_pool_withdraw_authority: self
                .instruction
                .stake_pool_withdraw_authority
                .expect("stake_pool_withdraw_authority is not set"),

            reserve_stake: self
                .instruction
                .reserve_stake
                .expect("reserve_stake is not set"),

            manager_fee_account: self
                .instruction
                .manager_fee_account
                .expect("manager_fee_account is not set"),

            referrer_pool_tokens_account: self
                .instruction
                .referrer_pool_tokens_account
                .expect("referrer_pool_tokens_account is not set"),

            pool_mint: self.instruction.pool_mint.expect("pool_mint is not set"),

            token_program: self
                .instruction
                .token_program
                .expect("token_program is not set"),

            system_program: self
                .instruction
                .system_program
                .expect("system_program is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct DistributeNcnOperatorRewardsCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    epoch_state: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    operator: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    operator_ata: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    operator_snapshot: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn_reward_router: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn_reward_receiver: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    restaking_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    stake_pool_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    stake_pool: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    stake_pool_withdraw_authority: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    reserve_stake: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    manager_fee_account: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    referrer_pool_tokens_account: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    pool_mint: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    token_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    system_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn_fee_group: Option<u8>,
    epoch: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

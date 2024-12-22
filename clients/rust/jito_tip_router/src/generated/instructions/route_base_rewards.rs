//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};

/// Accounts.
pub struct RouteBaseRewards {
    pub restaking_config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub epoch_snapshot: solana_program::pubkey::Pubkey,

    pub ballot_box: solana_program::pubkey::Pubkey,

    pub base_reward_router: solana_program::pubkey::Pubkey,

    pub base_reward_receiver: solana_program::pubkey::Pubkey,

    pub restaking_program: solana_program::pubkey::Pubkey,
}

impl RouteBaseRewards {
    pub fn instruction(
        &self,
        args: RouteBaseRewardsInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: RouteBaseRewardsInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(7 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.epoch_snapshot,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ballot_box,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.base_reward_router,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.base_reward_receiver,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = RouteBaseRewardsInstructionData::new().try_to_vec().unwrap();
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
pub struct RouteBaseRewardsInstructionData {
    discriminator: u8,
}

impl RouteBaseRewardsInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 13 }
    }
}

impl Default for RouteBaseRewardsInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RouteBaseRewardsInstructionArgs {
    pub max_iterations: u16,
    pub epoch: u64,
}

/// Instruction builder for `RouteBaseRewards`.
///
/// ### Accounts:
///
///   0. `[]` restaking_config
///   1. `[]` ncn
///   2. `[]` epoch_snapshot
///   3. `[]` ballot_box
///   4. `[writable]` base_reward_router
///   5. `[writable]` base_reward_receiver
///   6. `[]` restaking_program
#[derive(Clone, Debug, Default)]
pub struct RouteBaseRewardsBuilder {
    restaking_config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    epoch_snapshot: Option<solana_program::pubkey::Pubkey>,
    ballot_box: Option<solana_program::pubkey::Pubkey>,
    base_reward_router: Option<solana_program::pubkey::Pubkey>,
    base_reward_receiver: Option<solana_program::pubkey::Pubkey>,
    restaking_program: Option<solana_program::pubkey::Pubkey>,
    max_iterations: Option<u16>,
    epoch: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl RouteBaseRewardsBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn restaking_config(
        &mut self,
        restaking_config: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.restaking_config = Some(restaking_config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn epoch_snapshot(&mut self, epoch_snapshot: solana_program::pubkey::Pubkey) -> &mut Self {
        self.epoch_snapshot = Some(epoch_snapshot);
        self
    }
    #[inline(always)]
    pub fn ballot_box(&mut self, ballot_box: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ballot_box = Some(ballot_box);
        self
    }
    #[inline(always)]
    pub fn base_reward_router(
        &mut self,
        base_reward_router: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.base_reward_router = Some(base_reward_router);
        self
    }
    #[inline(always)]
    pub fn base_reward_receiver(
        &mut self,
        base_reward_receiver: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.base_reward_receiver = Some(base_reward_receiver);
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
    pub fn max_iterations(&mut self, max_iterations: u16) -> &mut Self {
        self.max_iterations = Some(max_iterations);
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
        let accounts = RouteBaseRewards {
            restaking_config: self.restaking_config.expect("restaking_config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            epoch_snapshot: self.epoch_snapshot.expect("epoch_snapshot is not set"),
            ballot_box: self.ballot_box.expect("ballot_box is not set"),
            base_reward_router: self
                .base_reward_router
                .expect("base_reward_router is not set"),
            base_reward_receiver: self
                .base_reward_receiver
                .expect("base_reward_receiver is not set"),
            restaking_program: self
                .restaking_program
                .expect("restaking_program is not set"),
        };
        let args = RouteBaseRewardsInstructionArgs {
            max_iterations: self
                .max_iterations
                .clone()
                .expect("max_iterations is not set"),
            epoch: self.epoch.clone().expect("epoch is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `route_base_rewards` CPI accounts.
pub struct RouteBaseRewardsCpiAccounts<'a, 'b> {
    pub restaking_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub epoch_snapshot: &'b solana_program::account_info::AccountInfo<'a>,

    pub ballot_box: &'b solana_program::account_info::AccountInfo<'a>,

    pub base_reward_router: &'b solana_program::account_info::AccountInfo<'a>,

    pub base_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `route_base_rewards` CPI instruction.
pub struct RouteBaseRewardsCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub epoch_snapshot: &'b solana_program::account_info::AccountInfo<'a>,

    pub ballot_box: &'b solana_program::account_info::AccountInfo<'a>,

    pub base_reward_router: &'b solana_program::account_info::AccountInfo<'a>,

    pub base_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: RouteBaseRewardsInstructionArgs,
}

impl<'a, 'b> RouteBaseRewardsCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: RouteBaseRewardsCpiAccounts<'a, 'b>,
        args: RouteBaseRewardsInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            restaking_config: accounts.restaking_config,
            ncn: accounts.ncn,
            epoch_snapshot: accounts.epoch_snapshot,
            ballot_box: accounts.ballot_box,
            base_reward_router: accounts.base_reward_router,
            base_reward_receiver: accounts.base_reward_receiver,
            restaking_program: accounts.restaking_program,
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
        let mut accounts = Vec::with_capacity(7 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.restaking_config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.epoch_snapshot.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ballot_box.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.base_reward_router.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.base_reward_receiver.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.restaking_program.key,
            false,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = RouteBaseRewardsInstructionData::new().try_to_vec().unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(7 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.restaking_config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.epoch_snapshot.clone());
        account_infos.push(self.ballot_box.clone());
        account_infos.push(self.base_reward_router.clone());
        account_infos.push(self.base_reward_receiver.clone());
        account_infos.push(self.restaking_program.clone());
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

/// Instruction builder for `RouteBaseRewards` via CPI.
///
/// ### Accounts:
///
///   0. `[]` restaking_config
///   1. `[]` ncn
///   2. `[]` epoch_snapshot
///   3. `[]` ballot_box
///   4. `[writable]` base_reward_router
///   5. `[writable]` base_reward_receiver
///   6. `[]` restaking_program
#[derive(Clone, Debug)]
pub struct RouteBaseRewardsCpiBuilder<'a, 'b> {
    instruction: Box<RouteBaseRewardsCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> RouteBaseRewardsCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(RouteBaseRewardsCpiBuilderInstruction {
            __program: program,
            restaking_config: None,
            ncn: None,
            epoch_snapshot: None,
            ballot_box: None,
            base_reward_router: None,
            base_reward_receiver: None,
            restaking_program: None,
            max_iterations: None,
            epoch: None,
            __remaining_accounts: Vec::new(),
        });
        Self { instruction }
    }
    #[inline(always)]
    pub fn restaking_config(
        &mut self,
        restaking_config: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.restaking_config = Some(restaking_config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn epoch_snapshot(
        &mut self,
        epoch_snapshot: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.epoch_snapshot = Some(epoch_snapshot);
        self
    }
    #[inline(always)]
    pub fn ballot_box(
        &mut self,
        ballot_box: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ballot_box = Some(ballot_box);
        self
    }
    #[inline(always)]
    pub fn base_reward_router(
        &mut self,
        base_reward_router: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.base_reward_router = Some(base_reward_router);
        self
    }
    #[inline(always)]
    pub fn base_reward_receiver(
        &mut self,
        base_reward_receiver: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.base_reward_receiver = Some(base_reward_receiver);
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
    pub fn max_iterations(&mut self, max_iterations: u16) -> &mut Self {
        self.instruction.max_iterations = Some(max_iterations);
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
        let args = RouteBaseRewardsInstructionArgs {
            max_iterations: self
                .instruction
                .max_iterations
                .clone()
                .expect("max_iterations is not set"),
            epoch: self.instruction.epoch.clone().expect("epoch is not set"),
        };
        let instruction = RouteBaseRewardsCpi {
            __program: self.instruction.__program,

            restaking_config: self
                .instruction
                .restaking_config
                .expect("restaking_config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            epoch_snapshot: self
                .instruction
                .epoch_snapshot
                .expect("epoch_snapshot is not set"),

            ballot_box: self.instruction.ballot_box.expect("ballot_box is not set"),

            base_reward_router: self
                .instruction
                .base_reward_router
                .expect("base_reward_router is not set"),

            base_reward_receiver: self
                .instruction
                .base_reward_receiver
                .expect("base_reward_receiver is not set"),

            restaking_program: self
                .instruction
                .restaking_program
                .expect("restaking_program is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct RouteBaseRewardsCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    restaking_config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    epoch_snapshot: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ballot_box: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    base_reward_router: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    base_reward_receiver: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    restaking_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    max_iterations: Option<u16>,
    epoch: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

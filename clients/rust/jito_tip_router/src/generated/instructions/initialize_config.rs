//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};

/// Accounts.
pub struct InitializeConfig {
    pub restaking_config: solana_program::pubkey::Pubkey,

    pub config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub fee_wallet: solana_program::pubkey::Pubkey,

    pub ncn_admin: solana_program::pubkey::Pubkey,

    pub tie_breaker_admin: solana_program::pubkey::Pubkey,

    pub restaking_program: solana_program::pubkey::Pubkey,

    pub system_program: solana_program::pubkey::Pubkey,
}

impl InitializeConfig {
    pub fn instruction(
        &self,
        args: InitializeConfigInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: InitializeConfigInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.fee_wallet,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn_admin,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.tie_breaker_admin,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.system_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = InitializeConfigInstructionData::new().try_to_vec().unwrap();
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
pub struct InitializeConfigInstructionData {
    discriminator: u8,
}

impl InitializeConfigInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 0 }
    }
}

impl Default for InitializeConfigInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitializeConfigInstructionArgs {
    pub block_engine_fee_bps: u16,
    pub dao_fee_bps: u16,
    pub default_ncn_fee_bps: u16,
    pub epochs_before_stall: u64,
    pub valid_slots_after_consensus: u64,
}

/// Instruction builder for `InitializeConfig`.
///
/// ### Accounts:
///
///   0. `[]` restaking_config
///   1. `[writable]` config
///   2. `[]` ncn
///   3. `[]` fee_wallet
///   4. `[signer]` ncn_admin
///   5. `[]` tie_breaker_admin
///   6. `[]` restaking_program
///   7. `[optional]` system_program (default to `11111111111111111111111111111111`)
#[derive(Clone, Debug, Default)]
pub struct InitializeConfigBuilder {
    restaking_config: Option<solana_program::pubkey::Pubkey>,
    config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    fee_wallet: Option<solana_program::pubkey::Pubkey>,
    ncn_admin: Option<solana_program::pubkey::Pubkey>,
    tie_breaker_admin: Option<solana_program::pubkey::Pubkey>,
    restaking_program: Option<solana_program::pubkey::Pubkey>,
    system_program: Option<solana_program::pubkey::Pubkey>,
    block_engine_fee_bps: Option<u16>,
    dao_fee_bps: Option<u16>,
    default_ncn_fee_bps: Option<u16>,
    epochs_before_stall: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl InitializeConfigBuilder {
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
    pub fn fee_wallet(&mut self, fee_wallet: solana_program::pubkey::Pubkey) -> &mut Self {
        self.fee_wallet = Some(fee_wallet);
        self
    }
    #[inline(always)]
    pub fn ncn_admin(&mut self, ncn_admin: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn_admin = Some(ncn_admin);
        self
    }
    #[inline(always)]
    pub fn tie_breaker_admin(
        &mut self,
        tie_breaker_admin: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.tie_breaker_admin = Some(tie_breaker_admin);
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
    /// `[optional account, default to '11111111111111111111111111111111']`
    #[inline(always)]
    pub fn system_program(&mut self, system_program: solana_program::pubkey::Pubkey) -> &mut Self {
        self.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn block_engine_fee_bps(&mut self, block_engine_fee_bps: u16) -> &mut Self {
        self.block_engine_fee_bps = Some(block_engine_fee_bps);
        self
    }
    #[inline(always)]
    pub fn dao_fee_bps(&mut self, dao_fee_bps: u16) -> &mut Self {
        self.dao_fee_bps = Some(dao_fee_bps);
        self
    }
    #[inline(always)]
    pub fn default_ncn_fee_bps(&mut self, default_ncn_fee_bps: u16) -> &mut Self {
        self.default_ncn_fee_bps = Some(default_ncn_fee_bps);
        self
    }
    #[inline(always)]
    pub fn epochs_before_stall(&mut self, epochs_before_stall: u64) -> &mut Self {
        self.epochs_before_stall = Some(epochs_before_stall);
        self
    }
    #[inline(always)]
    pub fn valid_slots_after_consensus(&mut self, valid_slots_after_consensus: u64) -> &mut Self {
        self.valid_slots_after_consensus = Some(valid_slots_after_consensus);
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
        let accounts = InitializeConfig {
            restaking_config: self.restaking_config.expect("restaking_config is not set"),
            config: self.config.expect("config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            fee_wallet: self.fee_wallet.expect("fee_wallet is not set"),
            ncn_admin: self.ncn_admin.expect("ncn_admin is not set"),
            tie_breaker_admin: self
                .tie_breaker_admin
                .expect("tie_breaker_admin is not set"),
            restaking_program: self
                .restaking_program
                .expect("restaking_program is not set"),
            system_program: self
                .system_program
                .unwrap_or(solana_program::pubkey!("11111111111111111111111111111111")),
        };
        let args = InitializeConfigInstructionArgs {
            block_engine_fee_bps: self
                .block_engine_fee_bps
                .clone()
                .expect("block_engine_fee_bps is not set"),
            dao_fee_bps: self.dao_fee_bps.clone().expect("dao_fee_bps is not set"),
            default_ncn_fee_bps: self
                .default_ncn_fee_bps
                .clone()
                .expect("default_ncn_fee_bps is not set"),
            epochs_before_stall: self
                .epochs_before_stall
                .clone()
                .expect("epochs_before_stall is not set"),
            valid_slots_after_consensus: self
                .valid_slots_after_consensus
                .clone()
                .expect("valid_slots_after_consensus is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `initialize_config` CPI accounts.
pub struct InitializeConfigCpiAccounts<'a, 'b> {
    pub restaking_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub fee_wallet: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,

    pub tie_breaker_admin: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `initialize_config` CPI instruction.
pub struct InitializeConfigCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub fee_wallet: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,

    pub tie_breaker_admin: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: InitializeConfigInstructionArgs,
}

impl<'a, 'b> InitializeConfigCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: InitializeConfigCpiAccounts<'a, 'b>,
        args: InitializeConfigInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            restaking_config: accounts.restaking_config,
            config: accounts.config,
            ncn: accounts.ncn,
            fee_wallet: accounts.fee_wallet,
            ncn_admin: accounts.ncn_admin,
            tie_breaker_admin: accounts.tie_breaker_admin,
            restaking_program: accounts.restaking_program,
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
        let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.restaking_config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.fee_wallet.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn_admin.key,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.tie_breaker_admin.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.restaking_program.key,
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
        let mut data = InitializeConfigInstructionData::new().try_to_vec().unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(8 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.restaking_config.clone());
        account_infos.push(self.config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.fee_wallet.clone());
        account_infos.push(self.ncn_admin.clone());
        account_infos.push(self.tie_breaker_admin.clone());
        account_infos.push(self.restaking_program.clone());
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

/// Instruction builder for `InitializeConfig` via CPI.
///
/// ### Accounts:
///
///   0. `[]` restaking_config
///   1. `[writable]` config
///   2. `[]` ncn
///   3. `[]` fee_wallet
///   4. `[signer]` ncn_admin
///   5. `[]` tie_breaker_admin
///   6. `[]` restaking_program
///   7. `[]` system_program
#[derive(Clone, Debug)]
pub struct InitializeConfigCpiBuilder<'a, 'b> {
    instruction: Box<InitializeConfigCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> InitializeConfigCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(InitializeConfigCpiBuilderInstruction {
            __program: program,
            restaking_config: None,
            config: None,
            ncn: None,
            fee_wallet: None,
            ncn_admin: None,
            tie_breaker_admin: None,
            restaking_program: None,
            system_program: None,
            block_engine_fee_bps: None,
            dao_fee_bps: None,
            default_ncn_fee_bps: None,
            epochs_before_stall: None,
            valid_slots_after_consensus: None,
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
    pub fn fee_wallet(
        &mut self,
        fee_wallet: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.fee_wallet = Some(fee_wallet);
        self
    }
    #[inline(always)]
    pub fn ncn_admin(
        &mut self,
        ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ncn_admin = Some(ncn_admin);
        self
    }
    #[inline(always)]
    pub fn tie_breaker_admin(
        &mut self,
        tie_breaker_admin: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.tie_breaker_admin = Some(tie_breaker_admin);
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
    pub fn system_program(
        &mut self,
        system_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn block_engine_fee_bps(&mut self, block_engine_fee_bps: u16) -> &mut Self {
        self.instruction.block_engine_fee_bps = Some(block_engine_fee_bps);
        self
    }
    #[inline(always)]
    pub fn dao_fee_bps(&mut self, dao_fee_bps: u16) -> &mut Self {
        self.instruction.dao_fee_bps = Some(dao_fee_bps);
        self
    }
    #[inline(always)]
    pub fn default_ncn_fee_bps(&mut self, default_ncn_fee_bps: u16) -> &mut Self {
        self.instruction.default_ncn_fee_bps = Some(default_ncn_fee_bps);
        self
    }
    #[inline(always)]
    pub fn epochs_before_stall(&mut self, epochs_before_stall: u64) -> &mut Self {
        self.instruction.epochs_before_stall = Some(epochs_before_stall);
        self
    }
    #[inline(always)]
    pub fn valid_slots_after_consensus(&mut self, valid_slots_after_consensus: u64) -> &mut Self {
        self.instruction.valid_slots_after_consensus = Some(valid_slots_after_consensus);
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
        let args = InitializeConfigInstructionArgs {
            block_engine_fee_bps: self
                .instruction
                .block_engine_fee_bps
                .clone()
                .expect("block_engine_fee_bps is not set"),
            dao_fee_bps: self
                .instruction
                .dao_fee_bps
                .clone()
                .expect("dao_fee_bps is not set"),
            default_ncn_fee_bps: self
                .instruction
                .default_ncn_fee_bps
                .clone()
                .expect("default_ncn_fee_bps is not set"),
            epochs_before_stall: self
                .instruction
                .epochs_before_stall
                .clone()
                .expect("epochs_before_stall is not set"),
            valid_slots_after_consensus: self
                .instruction
                .valid_slots_after_consensus
                .clone()
                .expect("valid_slots_after_consensus is not set"),
        };
        let instruction = InitializeConfigCpi {
            __program: self.instruction.__program,

            restaking_config: self
                .instruction
                .restaking_config
                .expect("restaking_config is not set"),

            config: self.instruction.config.expect("config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            fee_wallet: self.instruction.fee_wallet.expect("fee_wallet is not set"),

            ncn_admin: self.instruction.ncn_admin.expect("ncn_admin is not set"),

            tie_breaker_admin: self
                .instruction
                .tie_breaker_admin
                .expect("tie_breaker_admin is not set"),

            restaking_program: self
                .instruction
                .restaking_program
                .expect("restaking_program is not set"),

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
struct InitializeConfigCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    restaking_config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    fee_wallet: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn_admin: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    tie_breaker_admin: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    restaking_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    system_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    block_engine_fee_bps: Option<u16>,
    dao_fee_bps: Option<u16>,
    default_ncn_fee_bps: Option<u16>,
    epochs_before_stall: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

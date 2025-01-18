//! This code was AUTOGENERATED using the codama library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun codama to update it.
//!
//! <https://github.com/codama-idl/codama>
//!

use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;

/// Accounts.
pub struct AdminSetStMint {
    pub config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub vault_registry: solana_program::pubkey::Pubkey,

    pub admin: solana_program::pubkey::Pubkey,
}

impl AdminSetStMint {
    pub fn instruction(
        &self,
        args: AdminSetStMintInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: AdminSetStMintInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.vault_registry,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.admin, true,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = AdminSetStMintInstructionData::new().try_to_vec().unwrap();
        let mut args = args.try_to_vec().unwrap();
        data.append(&mut args);

        solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdminSetStMintInstructionData {
    discriminator: u8,
}

impl AdminSetStMintInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 33 }
    }
}

impl Default for AdminSetStMintInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdminSetStMintInstructionArgs {
    pub st_mint: Pubkey,
    pub ncn_fee_group: Option<u8>,
    pub reward_multiplier_bps: Option<u64>,
    pub switchboard_feed: Option<Pubkey>,
    pub no_feed_weight: Option<u128>,
}

/// Instruction builder for `AdminSetStMint`.
///
/// ### Accounts:
///
///   0. `[]` config
///   1. `[]` ncn
///   2. `[writable]` vault_registry
///   3. `[writable, signer]` admin
#[derive(Clone, Debug, Default)]
pub struct AdminSetStMintBuilder {
    config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    vault_registry: Option<solana_program::pubkey::Pubkey>,
    admin: Option<solana_program::pubkey::Pubkey>,
    st_mint: Option<Pubkey>,
    ncn_fee_group: Option<u8>,
    reward_multiplier_bps: Option<u64>,
    switchboard_feed: Option<Pubkey>,
    no_feed_weight: Option<u128>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl AdminSetStMintBuilder {
    pub fn new() -> Self {
        Self::default()
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
    pub fn vault_registry(&mut self, vault_registry: solana_program::pubkey::Pubkey) -> &mut Self {
        self.vault_registry = Some(vault_registry);
        self
    }
    #[inline(always)]
    pub fn admin(&mut self, admin: solana_program::pubkey::Pubkey) -> &mut Self {
        self.admin = Some(admin);
        self
    }
    #[inline(always)]
    pub fn st_mint(&mut self, st_mint: Pubkey) -> &mut Self {
        self.st_mint = Some(st_mint);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn ncn_fee_group(&mut self, ncn_fee_group: u8) -> &mut Self {
        self.ncn_fee_group = Some(ncn_fee_group);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn reward_multiplier_bps(&mut self, reward_multiplier_bps: u64) -> &mut Self {
        self.reward_multiplier_bps = Some(reward_multiplier_bps);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn switchboard_feed(&mut self, switchboard_feed: Pubkey) -> &mut Self {
        self.switchboard_feed = Some(switchboard_feed);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn no_feed_weight(&mut self, no_feed_weight: u128) -> &mut Self {
        self.no_feed_weight = Some(no_feed_weight);
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
        let accounts = AdminSetStMint {
            config: self.config.expect("config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            vault_registry: self.vault_registry.expect("vault_registry is not set"),
            admin: self.admin.expect("admin is not set"),
        };
        let args = AdminSetStMintInstructionArgs {
            st_mint: self.st_mint.clone().expect("st_mint is not set"),
            ncn_fee_group: self.ncn_fee_group.clone(),
            reward_multiplier_bps: self.reward_multiplier_bps.clone(),
            switchboard_feed: self.switchboard_feed.clone(),
            no_feed_weight: self.no_feed_weight.clone(),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `admin_set_st_mint` CPI accounts.
pub struct AdminSetStMintCpiAccounts<'a, 'b> {
    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub vault_registry: &'b solana_program::account_info::AccountInfo<'a>,

    pub admin: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `admin_set_st_mint` CPI instruction.
pub struct AdminSetStMintCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub vault_registry: &'b solana_program::account_info::AccountInfo<'a>,

    pub admin: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: AdminSetStMintInstructionArgs,
}

impl<'a, 'b> AdminSetStMintCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: AdminSetStMintCpiAccounts<'a, 'b>,
        args: AdminSetStMintInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            config: accounts.config,
            ncn: accounts.ncn,
            vault_registry: accounts.vault_registry,
            admin: accounts.admin,
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
        let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.vault_registry.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.admin.key,
            true,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = AdminSetStMintInstructionData::new().try_to_vec().unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(5 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.vault_registry.clone());
        account_infos.push(self.admin.clone());
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

/// Instruction builder for `AdminSetStMint` via CPI.
///
/// ### Accounts:
///
///   0. `[]` config
///   1. `[]` ncn
///   2. `[writable]` vault_registry
///   3. `[writable, signer]` admin
#[derive(Clone, Debug)]
pub struct AdminSetStMintCpiBuilder<'a, 'b> {
    instruction: Box<AdminSetStMintCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> AdminSetStMintCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(AdminSetStMintCpiBuilderInstruction {
            __program: program,
            config: None,
            ncn: None,
            vault_registry: None,
            admin: None,
            st_mint: None,
            ncn_fee_group: None,
            reward_multiplier_bps: None,
            switchboard_feed: None,
            no_feed_weight: None,
            __remaining_accounts: Vec::new(),
        });
        Self { instruction }
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
    pub fn vault_registry(
        &mut self,
        vault_registry: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.vault_registry = Some(vault_registry);
        self
    }
    #[inline(always)]
    pub fn admin(&mut self, admin: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.admin = Some(admin);
        self
    }
    #[inline(always)]
    pub fn st_mint(&mut self, st_mint: Pubkey) -> &mut Self {
        self.instruction.st_mint = Some(st_mint);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn ncn_fee_group(&mut self, ncn_fee_group: u8) -> &mut Self {
        self.instruction.ncn_fee_group = Some(ncn_fee_group);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn reward_multiplier_bps(&mut self, reward_multiplier_bps: u64) -> &mut Self {
        self.instruction.reward_multiplier_bps = Some(reward_multiplier_bps);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn switchboard_feed(&mut self, switchboard_feed: Pubkey) -> &mut Self {
        self.instruction.switchboard_feed = Some(switchboard_feed);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn no_feed_weight(&mut self, no_feed_weight: u128) -> &mut Self {
        self.instruction.no_feed_weight = Some(no_feed_weight);
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
        let args = AdminSetStMintInstructionArgs {
            st_mint: self
                .instruction
                .st_mint
                .clone()
                .expect("st_mint is not set"),
            ncn_fee_group: self.instruction.ncn_fee_group.clone(),
            reward_multiplier_bps: self.instruction.reward_multiplier_bps.clone(),
            switchboard_feed: self.instruction.switchboard_feed.clone(),
            no_feed_weight: self.instruction.no_feed_weight.clone(),
        };
        let instruction = AdminSetStMintCpi {
            __program: self.instruction.__program,

            config: self.instruction.config.expect("config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            vault_registry: self
                .instruction
                .vault_registry
                .expect("vault_registry is not set"),

            admin: self.instruction.admin.expect("admin is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct AdminSetStMintCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    vault_registry: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    admin: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    st_mint: Option<Pubkey>,
    ncn_fee_group: Option<u8>,
    reward_multiplier_bps: Option<u64>,
    switchboard_feed: Option<Pubkey>,
    no_feed_weight: Option<u128>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

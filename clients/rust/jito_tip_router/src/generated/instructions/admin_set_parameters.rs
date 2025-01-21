//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

use borsh::BorshDeserialize;
use borsh::BorshSerialize;

/// Accounts.
pub struct AdminSetParameters {
    pub config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub ncn_admin: solana_program::pubkey::Pubkey,
}

impl AdminSetParameters {
    pub fn instruction(
        &self,
        args: AdminSetParametersInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: AdminSetParametersInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(3 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn_admin,
            true,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = AdminSetParametersInstructionData::new()
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
pub struct AdminSetParametersInstructionData {
    discriminator: u8,
}

impl AdminSetParametersInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 28 }
    }
}

impl Default for AdminSetParametersInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdminSetParametersInstructionArgs {
    pub epochs_before_stall: Option<u64>,
    pub epochs_after_consensus_before_claim: Option<u64>,
    pub valid_slots_after_consensus: Option<u64>,
}

/// Instruction builder for `AdminSetParameters`.
///
/// ### Accounts:
///
///   0. `[writable]` config
///   1. `[]` ncn
///   2. `[signer]` ncn_admin
#[derive(Clone, Debug, Default)]
pub struct AdminSetParametersBuilder {
    config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    ncn_admin: Option<solana_program::pubkey::Pubkey>,
    epochs_before_stall: Option<u64>,
    epochs_after_consensus_before_claim: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl AdminSetParametersBuilder {
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
    pub fn ncn_admin(&mut self, ncn_admin: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn_admin = Some(ncn_admin);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn epochs_before_stall(&mut self, epochs_before_stall: u64) -> &mut Self {
        self.epochs_before_stall = Some(epochs_before_stall);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn epochs_after_consensus_before_claim(
        &mut self,
        epochs_after_consensus_before_claim: u64,
    ) -> &mut Self {
        self.epochs_after_consensus_before_claim = Some(epochs_after_consensus_before_claim);
        self
    }
    /// `[optional argument]`
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
        let accounts = AdminSetParameters {
            config: self.config.expect("config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            ncn_admin: self.ncn_admin.expect("ncn_admin is not set"),
        };
        let args = AdminSetParametersInstructionArgs {
            epochs_before_stall: self.epochs_before_stall.clone(),
            epochs_after_consensus_before_claim: self.epochs_after_consensus_before_claim.clone(),
            valid_slots_after_consensus: self.valid_slots_after_consensus.clone(),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `admin_set_parameters` CPI accounts.
pub struct AdminSetParametersCpiAccounts<'a, 'b> {
    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `admin_set_parameters` CPI instruction.
pub struct AdminSetParametersCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: AdminSetParametersInstructionArgs,
}

impl<'a, 'b> AdminSetParametersCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: AdminSetParametersCpiAccounts<'a, 'b>,
        args: AdminSetParametersInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            config: accounts.config,
            ncn: accounts.ncn,
            ncn_admin: accounts.ncn_admin,
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
        let mut accounts = Vec::with_capacity(3 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn_admin.key,
            true,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = AdminSetParametersInstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(3 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.ncn_admin.clone());
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

/// Instruction builder for `AdminSetParameters` via CPI.
///
/// ### Accounts:
///
///   0. `[writable]` config
///   1. `[]` ncn
///   2. `[signer]` ncn_admin
#[derive(Clone, Debug)]
pub struct AdminSetParametersCpiBuilder<'a, 'b> {
    instruction: Box<AdminSetParametersCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> AdminSetParametersCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(AdminSetParametersCpiBuilderInstruction {
            __program: program,
            config: None,
            ncn: None,
            ncn_admin: None,
            epochs_before_stall: None,
            epochs_after_consensus_before_claim: None,
            valid_slots_after_consensus: None,
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
    pub fn ncn_admin(
        &mut self,
        ncn_admin: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ncn_admin = Some(ncn_admin);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn epochs_before_stall(&mut self, epochs_before_stall: u64) -> &mut Self {
        self.instruction.epochs_before_stall = Some(epochs_before_stall);
        self
    }
    /// `[optional argument]`
    #[inline(always)]
    pub fn epochs_after_consensus_before_claim(
        &mut self,
        epochs_after_consensus_before_claim: u64,
    ) -> &mut Self {
        self.instruction.epochs_after_consensus_before_claim =
            Some(epochs_after_consensus_before_claim);
        self
    }
    /// `[optional argument]`
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
        let args = AdminSetParametersInstructionArgs {
            epochs_before_stall: self.instruction.epochs_before_stall.clone(),
            epochs_after_consensus_before_claim: self
                .instruction
                .epochs_after_consensus_before_claim
                .clone(),
            valid_slots_after_consensus: self.instruction.valid_slots_after_consensus.clone(),
        };
        let instruction = AdminSetParametersCpi {
            __program: self.instruction.__program,

            config: self.instruction.config.expect("config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            ncn_admin: self.instruction.ncn_admin.expect("ncn_admin is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct AdminSetParametersCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn_admin: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    epochs_before_stall: Option<u64>,
    epochs_after_consensus_before_claim: Option<u64>,
    valid_slots_after_consensus: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

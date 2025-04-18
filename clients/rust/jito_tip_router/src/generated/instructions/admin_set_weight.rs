//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

use borsh::BorshDeserialize;
use borsh::BorshSerialize;
use solana_program::pubkey::Pubkey;

/// Accounts.
pub struct AdminSetWeight {
    pub epoch_state: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub weight_table: solana_program::pubkey::Pubkey,

    pub weight_table_admin: solana_program::pubkey::Pubkey,
}

impl AdminSetWeight {
    pub fn instruction(
        &self,
        args: AdminSetWeightInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: AdminSetWeightInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.epoch_state,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.weight_table,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.weight_table_admin,
            true,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = AdminSetWeightInstructionData::new().try_to_vec().unwrap();
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
pub struct AdminSetWeightInstructionData {
    discriminator: u8,
}

impl AdminSetWeightInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 21 }
    }
}

impl Default for AdminSetWeightInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AdminSetWeightInstructionArgs {
    pub st_mint: Pubkey,
    pub weight: u128,
    pub epoch: u64,
}

/// Instruction builder for `AdminSetWeight`.
///
/// ### Accounts:
///
///   0. `[writable]` epoch_state
///   1. `[]` ncn
///   2. `[writable]` weight_table
///   3. `[signer]` weight_table_admin
#[derive(Clone, Debug, Default)]
pub struct AdminSetWeightBuilder {
    epoch_state: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    weight_table: Option<solana_program::pubkey::Pubkey>,
    weight_table_admin: Option<solana_program::pubkey::Pubkey>,
    st_mint: Option<Pubkey>,
    weight: Option<u128>,
    epoch: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl AdminSetWeightBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn epoch_state(&mut self, epoch_state: solana_program::pubkey::Pubkey) -> &mut Self {
        self.epoch_state = Some(epoch_state);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn weight_table(&mut self, weight_table: solana_program::pubkey::Pubkey) -> &mut Self {
        self.weight_table = Some(weight_table);
        self
    }
    #[inline(always)]
    pub fn weight_table_admin(
        &mut self,
        weight_table_admin: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.weight_table_admin = Some(weight_table_admin);
        self
    }
    #[inline(always)]
    pub fn st_mint(&mut self, st_mint: Pubkey) -> &mut Self {
        self.st_mint = Some(st_mint);
        self
    }
    #[inline(always)]
    pub fn weight(&mut self, weight: u128) -> &mut Self {
        self.weight = Some(weight);
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
        let accounts = AdminSetWeight {
            epoch_state: self.epoch_state.expect("epoch_state is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            weight_table: self.weight_table.expect("weight_table is not set"),
            weight_table_admin: self
                .weight_table_admin
                .expect("weight_table_admin is not set"),
        };
        let args = AdminSetWeightInstructionArgs {
            st_mint: self.st_mint.clone().expect("st_mint is not set"),
            weight: self.weight.clone().expect("weight is not set"),
            epoch: self.epoch.clone().expect("epoch is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `admin_set_weight` CPI accounts.
pub struct AdminSetWeightCpiAccounts<'a, 'b> {
    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table_admin: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `admin_set_weight` CPI instruction.
pub struct AdminSetWeightCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table_admin: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: AdminSetWeightInstructionArgs,
}

impl<'a, 'b> AdminSetWeightCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: AdminSetWeightCpiAccounts<'a, 'b>,
        args: AdminSetWeightInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            epoch_state: accounts.epoch_state,
            ncn: accounts.ncn,
            weight_table: accounts.weight_table,
            weight_table_admin: accounts.weight_table_admin,
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
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.epoch_state.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.weight_table.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.weight_table_admin.key,
            true,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = AdminSetWeightInstructionData::new().try_to_vec().unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(4 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.epoch_state.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.weight_table.clone());
        account_infos.push(self.weight_table_admin.clone());
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

/// Instruction builder for `AdminSetWeight` via CPI.
///
/// ### Accounts:
///
///   0. `[writable]` epoch_state
///   1. `[]` ncn
///   2. `[writable]` weight_table
///   3. `[signer]` weight_table_admin
#[derive(Clone, Debug)]
pub struct AdminSetWeightCpiBuilder<'a, 'b> {
    instruction: Box<AdminSetWeightCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> AdminSetWeightCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(AdminSetWeightCpiBuilderInstruction {
            __program: program,
            epoch_state: None,
            ncn: None,
            weight_table: None,
            weight_table_admin: None,
            st_mint: None,
            weight: None,
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
    pub fn ncn(&mut self, ncn: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn weight_table(
        &mut self,
        weight_table: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.weight_table = Some(weight_table);
        self
    }
    #[inline(always)]
    pub fn weight_table_admin(
        &mut self,
        weight_table_admin: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.weight_table_admin = Some(weight_table_admin);
        self
    }
    #[inline(always)]
    pub fn st_mint(&mut self, st_mint: Pubkey) -> &mut Self {
        self.instruction.st_mint = Some(st_mint);
        self
    }
    #[inline(always)]
    pub fn weight(&mut self, weight: u128) -> &mut Self {
        self.instruction.weight = Some(weight);
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
        let args = AdminSetWeightInstructionArgs {
            st_mint: self
                .instruction
                .st_mint
                .clone()
                .expect("st_mint is not set"),
            weight: self.instruction.weight.clone().expect("weight is not set"),
            epoch: self.instruction.epoch.clone().expect("epoch is not set"),
        };
        let instruction = AdminSetWeightCpi {
            __program: self.instruction.__program,

            epoch_state: self
                .instruction
                .epoch_state
                .expect("epoch_state is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            weight_table: self
                .instruction
                .weight_table
                .expect("weight_table is not set"),

            weight_table_admin: self
                .instruction
                .weight_table_admin
                .expect("weight_table_admin is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct AdminSetWeightCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    epoch_state: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    weight_table: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    weight_table_admin: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    st_mint: Option<Pubkey>,
    weight: Option<u128>,
    epoch: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

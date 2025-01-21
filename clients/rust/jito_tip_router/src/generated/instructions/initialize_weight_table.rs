//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>
//!

use borsh::BorshDeserialize;
use borsh::BorshSerialize;

/// Accounts.
pub struct InitializeWeightTable {
    pub epoch_state: solana_program::pubkey::Pubkey,

    pub vault_registry: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub weight_table: solana_program::pubkey::Pubkey,

    pub account_payer: solana_program::pubkey::Pubkey,

    pub system_program: solana_program::pubkey::Pubkey,
}

impl InitializeWeightTable {
    pub fn instruction(
        &self,
        args: InitializeWeightTableInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: InitializeWeightTableInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(6 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.epoch_state,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.vault_registry,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.weight_table,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.account_payer,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.system_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = InitializeWeightTableInstructionData::new()
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
pub struct InitializeWeightTableInstructionData {
    discriminator: u8,
}

impl InitializeWeightTableInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 6 }
    }
}

impl Default for InitializeWeightTableInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitializeWeightTableInstructionArgs {
    pub epoch: u64,
}

/// Instruction builder for `InitializeWeightTable`.
///
/// ### Accounts:
///
///   0. `[]` epoch_state
///   1. `[]` vault_registry
///   2. `[]` ncn
///   3. `[writable]` weight_table
///   4. `[writable]` account_payer
///   5. `[optional]` system_program (default to `11111111111111111111111111111111`)
#[derive(Clone, Debug, Default)]
pub struct InitializeWeightTableBuilder {
    epoch_state: Option<solana_program::pubkey::Pubkey>,
    vault_registry: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    weight_table: Option<solana_program::pubkey::Pubkey>,
    account_payer: Option<solana_program::pubkey::Pubkey>,
    system_program: Option<solana_program::pubkey::Pubkey>,
    epoch: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl InitializeWeightTableBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn epoch_state(&mut self, epoch_state: solana_program::pubkey::Pubkey) -> &mut Self {
        self.epoch_state = Some(epoch_state);
        self
    }
    #[inline(always)]
    pub fn vault_registry(&mut self, vault_registry: solana_program::pubkey::Pubkey) -> &mut Self {
        self.vault_registry = Some(vault_registry);
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
    pub fn account_payer(&mut self, account_payer: solana_program::pubkey::Pubkey) -> &mut Self {
        self.account_payer = Some(account_payer);
        self
    }
    /// `[optional account, default to '11111111111111111111111111111111']`
    #[inline(always)]
    pub fn system_program(&mut self, system_program: solana_program::pubkey::Pubkey) -> &mut Self {
        self.system_program = Some(system_program);
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
        let accounts = InitializeWeightTable {
            epoch_state: self.epoch_state.expect("epoch_state is not set"),
            vault_registry: self.vault_registry.expect("vault_registry is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            weight_table: self.weight_table.expect("weight_table is not set"),
            account_payer: self.account_payer.expect("account_payer is not set"),
            system_program: self
                .system_program
                .unwrap_or(solana_program::pubkey!("11111111111111111111111111111111")),
        };
        let args = InitializeWeightTableInstructionArgs {
            epoch: self.epoch.clone().expect("epoch is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `initialize_weight_table` CPI accounts.
pub struct InitializeWeightTableCpiAccounts<'a, 'b> {
    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub vault_registry: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table: &'b solana_program::account_info::AccountInfo<'a>,

    pub account_payer: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `initialize_weight_table` CPI instruction.
pub struct InitializeWeightTableCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub epoch_state: &'b solana_program::account_info::AccountInfo<'a>,

    pub vault_registry: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub weight_table: &'b solana_program::account_info::AccountInfo<'a>,

    pub account_payer: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: InitializeWeightTableInstructionArgs,
}

impl<'a, 'b> InitializeWeightTableCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: InitializeWeightTableCpiAccounts<'a, 'b>,
        args: InitializeWeightTableInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            epoch_state: accounts.epoch_state,
            vault_registry: accounts.vault_registry,
            ncn: accounts.ncn,
            weight_table: accounts.weight_table,
            account_payer: accounts.account_payer,
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
        let mut accounts = Vec::with_capacity(6 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.epoch_state.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.vault_registry.key,
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
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.account_payer.key,
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
        let mut data = InitializeWeightTableInstructionData::new()
            .try_to_vec()
            .unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(6 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.epoch_state.clone());
        account_infos.push(self.vault_registry.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.weight_table.clone());
        account_infos.push(self.account_payer.clone());
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

/// Instruction builder for `InitializeWeightTable` via CPI.
///
/// ### Accounts:
///
///   0. `[]` epoch_state
///   1. `[]` vault_registry
///   2. `[]` ncn
///   3. `[writable]` weight_table
///   4. `[writable]` account_payer
///   5. `[]` system_program
#[derive(Clone, Debug)]
pub struct InitializeWeightTableCpiBuilder<'a, 'b> {
    instruction: Box<InitializeWeightTableCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> InitializeWeightTableCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(InitializeWeightTableCpiBuilderInstruction {
            __program: program,
            epoch_state: None,
            vault_registry: None,
            ncn: None,
            weight_table: None,
            account_payer: None,
            system_program: None,
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
    pub fn vault_registry(
        &mut self,
        vault_registry: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.vault_registry = Some(vault_registry);
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
    pub fn account_payer(
        &mut self,
        account_payer: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.account_payer = Some(account_payer);
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
        let args = InitializeWeightTableInstructionArgs {
            epoch: self.instruction.epoch.clone().expect("epoch is not set"),
        };
        let instruction = InitializeWeightTableCpi {
            __program: self.instruction.__program,

            epoch_state: self
                .instruction
                .epoch_state
                .expect("epoch_state is not set"),

            vault_registry: self
                .instruction
                .vault_registry
                .expect("vault_registry is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            weight_table: self
                .instruction
                .weight_table
                .expect("weight_table is not set"),

            account_payer: self
                .instruction
                .account_payer
                .expect("account_payer is not set"),

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
struct InitializeWeightTableCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    epoch_state: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    vault_registry: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    weight_table: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    account_payer: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    system_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    epoch: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}

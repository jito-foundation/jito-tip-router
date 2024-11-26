//! This code was AUTOGENERATED using the kinobi library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun kinobi to update it.
//!
//! <https://github.com/kinobi-so/kinobi>

use borsh::{BorshDeserialize, BorshSerialize};

/// Accounts.
pub struct SetMerkleRoot {
    pub ncn_config: solana_program::pubkey::Pubkey,

    pub ncn: solana_program::pubkey::Pubkey,

    pub ballot_box: solana_program::pubkey::Pubkey,

    pub vote_account: solana_program::pubkey::Pubkey,

    pub tip_distribution_account: solana_program::pubkey::Pubkey,

    pub tip_distribution_config: solana_program::pubkey::Pubkey,

    pub tip_distribution_program: solana_program::pubkey::Pubkey,

    pub restaking_program: solana_program::pubkey::Pubkey,
}

impl SetMerkleRoot {
    pub fn instruction(
        &self,
        args: SetMerkleRootInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: SetMerkleRootInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.ncn_config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ncn, false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.ballot_box,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.vote_account,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.tip_distribution_account,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.tip_distribution_config,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.tip_distribution_program,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.restaking_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = SetMerkleRootInstructionData::new().try_to_vec().unwrap();
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
pub struct SetMerkleRootInstructionData {
    discriminator: u8,
}

impl SetMerkleRootInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 12 }
    }
}

impl Default for SetMerkleRootInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SetMerkleRootInstructionArgs {
    pub proof: Vec<[u8; 32]>,
    pub merkle_root: [u8; 32],
    pub max_total_claim: u64,
    pub max_num_nodes: u64,
    pub epoch: u64,
}

/// Instruction builder for `SetMerkleRoot`.
///
/// ### Accounts:
///
///   0. `[writable]` ncn_config
///   1. `[]` ncn
///   2. `[]` ballot_box
///   3. `[]` vote_account
///   4. `[writable]` tip_distribution_account
///   5. `[]` tip_distribution_config
///   6. `[]` tip_distribution_program
///   7. `[]` restaking_program
#[derive(Clone, Debug, Default)]
pub struct SetMerkleRootBuilder {
    ncn_config: Option<solana_program::pubkey::Pubkey>,
    ncn: Option<solana_program::pubkey::Pubkey>,
    ballot_box: Option<solana_program::pubkey::Pubkey>,
    vote_account: Option<solana_program::pubkey::Pubkey>,
    tip_distribution_account: Option<solana_program::pubkey::Pubkey>,
    tip_distribution_config: Option<solana_program::pubkey::Pubkey>,
    tip_distribution_program: Option<solana_program::pubkey::Pubkey>,
    restaking_program: Option<solana_program::pubkey::Pubkey>,
    proof: Option<Vec<[u8; 32]>>,
    merkle_root: Option<[u8; 32]>,
    max_total_claim: Option<u64>,
    max_num_nodes: Option<u64>,
    epoch: Option<u64>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl SetMerkleRootBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn ncn_config(&mut self, ncn_config: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn_config = Some(ncn_config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ncn = Some(ncn);
        self
    }
    #[inline(always)]
    pub fn ballot_box(&mut self, ballot_box: solana_program::pubkey::Pubkey) -> &mut Self {
        self.ballot_box = Some(ballot_box);
        self
    }
    #[inline(always)]
    pub fn vote_account(&mut self, vote_account: solana_program::pubkey::Pubkey) -> &mut Self {
        self.vote_account = Some(vote_account);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_account(
        &mut self,
        tip_distribution_account: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.tip_distribution_account = Some(tip_distribution_account);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_config(
        &mut self,
        tip_distribution_config: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.tip_distribution_config = Some(tip_distribution_config);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_program(
        &mut self,
        tip_distribution_program: solana_program::pubkey::Pubkey,
    ) -> &mut Self {
        self.tip_distribution_program = Some(tip_distribution_program);
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
    pub fn proof(&mut self, proof: Vec<[u8; 32]>) -> &mut Self {
        self.proof = Some(proof);
        self
    }
    #[inline(always)]
    pub fn merkle_root(&mut self, merkle_root: [u8; 32]) -> &mut Self {
        self.merkle_root = Some(merkle_root);
        self
    }
    #[inline(always)]
    pub fn max_total_claim(&mut self, max_total_claim: u64) -> &mut Self {
        self.max_total_claim = Some(max_total_claim);
        self
    }
    #[inline(always)]
    pub fn max_num_nodes(&mut self, max_num_nodes: u64) -> &mut Self {
        self.max_num_nodes = Some(max_num_nodes);
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
        let accounts = SetMerkleRoot {
            ncn_config: self.ncn_config.expect("ncn_config is not set"),
            ncn: self.ncn.expect("ncn is not set"),
            ballot_box: self.ballot_box.expect("ballot_box is not set"),
            vote_account: self.vote_account.expect("vote_account is not set"),
            tip_distribution_account: self
                .tip_distribution_account
                .expect("tip_distribution_account is not set"),
            tip_distribution_config: self
                .tip_distribution_config
                .expect("tip_distribution_config is not set"),
            tip_distribution_program: self
                .tip_distribution_program
                .expect("tip_distribution_program is not set"),
            restaking_program: self
                .restaking_program
                .expect("restaking_program is not set"),
        };
        let args = SetMerkleRootInstructionArgs {
            proof: self.proof.clone().expect("proof is not set"),
            merkle_root: self.merkle_root.clone().expect("merkle_root is not set"),
            max_total_claim: self
                .max_total_claim
                .clone()
                .expect("max_total_claim is not set"),
            max_num_nodes: self
                .max_num_nodes
                .clone()
                .expect("max_num_nodes is not set"),
            epoch: self.epoch.clone().expect("epoch is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `set_merkle_root` CPI accounts.
pub struct SetMerkleRootCpiAccounts<'a, 'b> {
    pub ncn_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub ballot_box: &'b solana_program::account_info::AccountInfo<'a>,

    pub vote_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `set_merkle_root` CPI instruction.
pub struct SetMerkleRootCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub ncn: &'b solana_program::account_info::AccountInfo<'a>,

    pub ballot_box: &'b solana_program::account_info::AccountInfo<'a>,

    pub vote_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_account: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_config: &'b solana_program::account_info::AccountInfo<'a>,

    pub tip_distribution_program: &'b solana_program::account_info::AccountInfo<'a>,

    pub restaking_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: SetMerkleRootInstructionArgs,
}

impl<'a, 'b> SetMerkleRootCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: SetMerkleRootCpiAccounts<'a, 'b>,
        args: SetMerkleRootInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            ncn_config: accounts.ncn_config,
            ncn: accounts.ncn,
            ballot_box: accounts.ballot_box,
            vote_account: accounts.vote_account,
            tip_distribution_account: accounts.tip_distribution_account,
            tip_distribution_config: accounts.tip_distribution_config,
            tip_distribution_program: accounts.tip_distribution_program,
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
        let mut accounts = Vec::with_capacity(8 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.ncn_config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ncn.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.ballot_box.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.vote_account.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.tip_distribution_account.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.tip_distribution_config.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.tip_distribution_program.key,
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
        let mut data = SetMerkleRootInstructionData::new().try_to_vec().unwrap();
        let mut args = self.__args.try_to_vec().unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::JITO_TIP_ROUTER_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(8 + 1 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.ncn_config.clone());
        account_infos.push(self.ncn.clone());
        account_infos.push(self.ballot_box.clone());
        account_infos.push(self.vote_account.clone());
        account_infos.push(self.tip_distribution_account.clone());
        account_infos.push(self.tip_distribution_config.clone());
        account_infos.push(self.tip_distribution_program.clone());
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

/// Instruction builder for `SetMerkleRoot` via CPI.
///
/// ### Accounts:
///
///   0. `[writable]` ncn_config
///   1. `[]` ncn
///   2. `[]` ballot_box
///   3. `[]` vote_account
///   4. `[writable]` tip_distribution_account
///   5. `[]` tip_distribution_config
///   6. `[]` tip_distribution_program
///   7. `[]` restaking_program
#[derive(Clone, Debug)]
pub struct SetMerkleRootCpiBuilder<'a, 'b> {
    instruction: Box<SetMerkleRootCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> SetMerkleRootCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(SetMerkleRootCpiBuilderInstruction {
            __program: program,
            ncn_config: None,
            ncn: None,
            ballot_box: None,
            vote_account: None,
            tip_distribution_account: None,
            tip_distribution_config: None,
            tip_distribution_program: None,
            restaking_program: None,
            proof: None,
            merkle_root: None,
            max_total_claim: None,
            max_num_nodes: None,
            epoch: None,
            __remaining_accounts: Vec::new(),
        });
        Self { instruction }
    }
    #[inline(always)]
    pub fn ncn_config(
        &mut self,
        ncn_config: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.ncn_config = Some(ncn_config);
        self
    }
    #[inline(always)]
    pub fn ncn(&mut self, ncn: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.ncn = Some(ncn);
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
    pub fn vote_account(
        &mut self,
        vote_account: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.vote_account = Some(vote_account);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_account(
        &mut self,
        tip_distribution_account: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.tip_distribution_account = Some(tip_distribution_account);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_config(
        &mut self,
        tip_distribution_config: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.tip_distribution_config = Some(tip_distribution_config);
        self
    }
    #[inline(always)]
    pub fn tip_distribution_program(
        &mut self,
        tip_distribution_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.tip_distribution_program = Some(tip_distribution_program);
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
    pub fn proof(&mut self, proof: Vec<[u8; 32]>) -> &mut Self {
        self.instruction.proof = Some(proof);
        self
    }
    #[inline(always)]
    pub fn merkle_root(&mut self, merkle_root: [u8; 32]) -> &mut Self {
        self.instruction.merkle_root = Some(merkle_root);
        self
    }
    #[inline(always)]
    pub fn max_total_claim(&mut self, max_total_claim: u64) -> &mut Self {
        self.instruction.max_total_claim = Some(max_total_claim);
        self
    }
    #[inline(always)]
    pub fn max_num_nodes(&mut self, max_num_nodes: u64) -> &mut Self {
        self.instruction.max_num_nodes = Some(max_num_nodes);
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
        let args = SetMerkleRootInstructionArgs {
            proof: self.instruction.proof.clone().expect("proof is not set"),
            merkle_root: self
                .instruction
                .merkle_root
                .clone()
                .expect("merkle_root is not set"),
            max_total_claim: self
                .instruction
                .max_total_claim
                .clone()
                .expect("max_total_claim is not set"),
            max_num_nodes: self
                .instruction
                .max_num_nodes
                .clone()
                .expect("max_num_nodes is not set"),
            epoch: self.instruction.epoch.clone().expect("epoch is not set"),
        };
        let instruction = SetMerkleRootCpi {
            __program: self.instruction.__program,

            ncn_config: self.instruction.ncn_config.expect("ncn_config is not set"),

            ncn: self.instruction.ncn.expect("ncn is not set"),

            ballot_box: self.instruction.ballot_box.expect("ballot_box is not set"),

            vote_account: self
                .instruction
                .vote_account
                .expect("vote_account is not set"),

            tip_distribution_account: self
                .instruction
                .tip_distribution_account
                .expect("tip_distribution_account is not set"),

            tip_distribution_config: self
                .instruction
                .tip_distribution_config
                .expect("tip_distribution_config is not set"),

            tip_distribution_program: self
                .instruction
                .tip_distribution_program
                .expect("tip_distribution_program is not set"),

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
struct SetMerkleRootCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    ncn_config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ncn: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    ballot_box: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    vote_account: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    tip_distribution_account: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    tip_distribution_config: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    tip_distribution_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    restaking_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    proof: Option<Vec<[u8; 32]>>,
    merkle_root: Option<[u8; 32]>,
    max_total_claim: Option<u64>,
    max_num_nodes: Option<u64>,
    epoch: Option<u64>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}
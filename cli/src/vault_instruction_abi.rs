use anyhow::{anyhow, Result};
use solana_sdk::instruction::Instruction;

/// The Vault program instruction layout expected by the deployed program.
///
/// This is a temporary, testnet-only compatibility shim. The Vault program currently deployed on
/// testnet predates the insertion of `RevokeDelegateTokenAccount`, while this repository's pinned
/// Vault client includes it and therefore emits instruction enum ordinals that are one greater for
/// the three vault-update instructions below. Legacy mode rewrites only those ordinals so the
/// testnet keeper can update vaults until the testnet Vault program is upgraded. It must never be
/// enabled on mainnet or on any deployment whose Vault program matches the current client ABI, and
/// should be removed after the testnet upgrade is complete.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum VaultInstructionAbi {
    /// Instruction layout at and after the addition of `RevokeDelegateTokenAccount`.
    #[default]
    Current,
    /// Instruction layout before `RevokeDelegateTokenAccount` shifted later enum variants.
    LegacyPreRevoke,
}

impl VaultInstructionAbi {
    pub(crate) fn adapt_vault_update_instruction(
        self,
        mut instruction: Instruction,
        vault_update_instruction: VaultUpdateInstruction,
    ) -> Result<Instruction> {
        let discriminator = instruction
            .data
            .first_mut()
            .ok_or_else(|| anyhow!("vault update instruction data is empty"))?;
        let expected_current_discriminator = vault_update_instruction.current_discriminator();

        if *discriminator != expected_current_discriminator {
            return Err(anyhow!(
                "unexpected current vault instruction discriminator: expected {}, found {}",
                expected_current_discriminator,
                discriminator
            ));
        }

        if self == Self::LegacyPreRevoke {
            *discriminator = vault_update_instruction.legacy_discriminator();
        }

        Ok(instruction)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum VaultUpdateInstruction {
    Initialize,
    Crank,
    Close,
}

impl VaultUpdateInstruction {
    const fn current_discriminator(self) -> u8 {
        match self {
            Self::Initialize => 27,
            Self::Crank => 28,
            Self::Close => 29,
        }
    }

    const fn legacy_discriminator(self) -> u8 {
        match self {
            Self::Initialize => 26,
            Self::Crank => 27,
            Self::Close => 28,
        }
    }
}

#[cfg(test)]
mod tests {
    use solana_sdk::{instruction::AccountMeta, pubkey::Pubkey};

    use super::*;

    fn instruction_with_data(data: Vec<u8>) -> Instruction {
        Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![AccountMeta::new(Pubkey::new_unique(), true)],
            data,
        }
    }

    #[test]
    fn legacy_abi_rewrites_only_vault_update_discriminators() {
        let cases = [
            (VaultUpdateInstruction::Initialize, vec![27, 0], vec![26, 0]),
            (VaultUpdateInstruction::Crank, vec![28], vec![27]),
            (
                VaultUpdateInstruction::Close,
                vec![29, 42, 0, 0, 0, 0, 0, 0, 0],
                vec![28, 42, 0, 0, 0, 0, 0, 0, 0],
            ),
        ];

        for (vault_update_instruction, current_data, legacy_data) in cases {
            let current_instruction = instruction_with_data(current_data);
            let program_id = current_instruction.program_id;
            let accounts = current_instruction.accounts.clone();

            let legacy_instruction = VaultInstructionAbi::LegacyPreRevoke
                .adapt_vault_update_instruction(current_instruction, vault_update_instruction)
                .unwrap();

            assert_eq!(legacy_instruction.program_id, program_id);
            assert_eq!(legacy_instruction.accounts, accounts);
            assert_eq!(legacy_instruction.data, legacy_data);
        }
    }

    #[test]
    fn current_abi_leaves_instruction_unchanged() {
        let instruction = instruction_with_data(vec![27, 0]);
        let expected = instruction.clone();

        let adapted = VaultInstructionAbi::Current
            .adapt_vault_update_instruction(instruction, VaultUpdateInstruction::Initialize)
            .unwrap();

        assert_eq!(adapted, expected);
    }

    #[test]
    fn adapter_rejects_unexpected_or_empty_instruction_data() {
        let unexpected = instruction_with_data(vec![30]);
        let empty = instruction_with_data(Vec::new());

        assert!(VaultInstructionAbi::LegacyPreRevoke
            .adapt_vault_update_instruction(unexpected, VaultUpdateInstruction::Initialize)
            .is_err());
        assert!(VaultInstructionAbi::LegacyPreRevoke
            .adapt_vault_update_instruction(empty, VaultUpdateInstruction::Initialize)
            .is_err());
    }
}

/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU8Decoder,
  getU8Encoder,
  transformEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type IAccountMeta,
  type IAccountSignerMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type ReadonlyAccount,
  type TransactionSigner,
  type WritableAccount,
  type WritableSignerAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const REALLOC_VAULT_REGISTRY_DISCRIMINATOR = 2;

export function getReallocVaultRegistryDiscriminatorBytes() {
  return getU8Encoder().encode(REALLOC_VAULT_REGISTRY_DISCRIMINATOR);
}

export type ReallocVaultRegistryInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountVaultRegistry extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountPayer extends string | IAccountMeta<string> = string,
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountVaultRegistry extends string
        ? WritableAccount<TAccountVaultRegistry>
        : TAccountVaultRegistry,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountPayer extends string
        ? WritableSignerAccount<TAccountPayer> &
            IAccountSignerMeta<TAccountPayer>
        : TAccountPayer,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type ReallocVaultRegistryInstructionData = { discriminator: number };

export type ReallocVaultRegistryInstructionDataArgs = {};

export function getReallocVaultRegistryInstructionDataEncoder(): Encoder<ReallocVaultRegistryInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([['discriminator', getU8Encoder()]]),
    (value) => ({
      ...value,
      discriminator: REALLOC_VAULT_REGISTRY_DISCRIMINATOR,
    })
  );
}

export function getReallocVaultRegistryInstructionDataDecoder(): Decoder<ReallocVaultRegistryInstructionData> {
  return getStructDecoder([['discriminator', getU8Decoder()]]);
}

export function getReallocVaultRegistryInstructionDataCodec(): Codec<
  ReallocVaultRegistryInstructionDataArgs,
  ReallocVaultRegistryInstructionData
> {
  return combineCodec(
    getReallocVaultRegistryInstructionDataEncoder(),
    getReallocVaultRegistryInstructionDataDecoder()
  );
}

export type ReallocVaultRegistryInput<
  TAccountConfig extends string = string,
  TAccountVaultRegistry extends string = string,
  TAccountNcn extends string = string,
  TAccountPayer extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  config: Address<TAccountConfig>;
  vaultRegistry: Address<TAccountVaultRegistry>;
  ncn: Address<TAccountNcn>;
  payer: TransactionSigner<TAccountPayer>;
  systemProgram?: Address<TAccountSystemProgram>;
};

export function getReallocVaultRegistryInstruction<
  TAccountConfig extends string,
  TAccountVaultRegistry extends string,
  TAccountNcn extends string,
  TAccountPayer extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: ReallocVaultRegistryInput<
    TAccountConfig,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountPayer,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): ReallocVaultRegistryInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountVaultRegistry,
  TAccountNcn,
  TAccountPayer,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: false },
    vaultRegistry: { value: input.vaultRegistry ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    payer: { value: input.payer ?? null, isWritable: true },
    systemProgram: { value: input.systemProgram ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Resolve default values.
  if (!accounts.systemProgram.value) {
    accounts.systemProgram.value =
      '11111111111111111111111111111111' as Address<'11111111111111111111111111111111'>;
  }

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.vaultRegistry),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getReallocVaultRegistryInstructionDataEncoder().encode({}),
  } as ReallocVaultRegistryInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountPayer,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedReallocVaultRegistryInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    vaultRegistry: TAccountMetas[1];
    ncn: TAccountMetas[2];
    payer: TAccountMetas[3];
    systemProgram: TAccountMetas[4];
  };
  data: ReallocVaultRegistryInstructionData;
};

export function parseReallocVaultRegistryInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedReallocVaultRegistryInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 5) {
    // TODO: Coded error.
    throw new Error('Not enough accounts');
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts![accountIndex]!;
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      config: getNextAccount(),
      vaultRegistry: getNextAccount(),
      ncn: getNextAccount(),
      payer: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getReallocVaultRegistryInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

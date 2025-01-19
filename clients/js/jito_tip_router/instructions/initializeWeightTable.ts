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
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  transformEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type IAccountMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type ReadonlyAccount,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR = 6;

export function getInitializeWeightTableDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR);
}

export type InitializeWeightTableInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountEpochState extends string | IAccountMeta<string> = string,
  TAccountVaultRegistry extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountClaimStatusPayer extends string | IAccountMeta<string> = string,
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountEpochState extends string
        ? ReadonlyAccount<TAccountEpochState>
        : TAccountEpochState,
      TAccountVaultRegistry extends string
        ? ReadonlyAccount<TAccountVaultRegistry>
        : TAccountVaultRegistry,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountClaimStatusPayer extends string
        ? WritableAccount<TAccountClaimStatusPayer>
        : TAccountClaimStatusPayer,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type InitializeWeightTableInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type InitializeWeightTableInstructionDataArgs = {
  epoch: number | bigint;
};

export function getInitializeWeightTableInstructionDataEncoder(): Encoder<InitializeWeightTableInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR,
    })
  );
}

export function getInitializeWeightTableInstructionDataDecoder(): Decoder<InitializeWeightTableInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getInitializeWeightTableInstructionDataCodec(): Codec<
  InitializeWeightTableInstructionDataArgs,
  InitializeWeightTableInstructionData
> {
  return combineCodec(
    getInitializeWeightTableInstructionDataEncoder(),
    getInitializeWeightTableInstructionDataDecoder()
  );
}

export type InitializeWeightTableInput<
  TAccountEpochState extends string = string,
  TAccountVaultRegistry extends string = string,
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountClaimStatusPayer extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  epochState: Address<TAccountEpochState>;
  vaultRegistry: Address<TAccountVaultRegistry>;
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  claimStatusPayer: Address<TAccountClaimStatusPayer>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: InitializeWeightTableInstructionDataArgs['epoch'];
};

export function getInitializeWeightTableInstruction<
  TAccountEpochState extends string,
  TAccountVaultRegistry extends string,
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountClaimStatusPayer extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeWeightTableInput<
    TAccountEpochState,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountWeightTable,
    TAccountClaimStatusPayer,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeWeightTableInstruction<
  TProgramAddress,
  TAccountEpochState,
  TAccountVaultRegistry,
  TAccountNcn,
  TAccountWeightTable,
  TAccountClaimStatusPayer,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    epochState: { value: input.epochState ?? null, isWritable: false },
    vaultRegistry: { value: input.vaultRegistry ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
    claimStatusPayer: {
      value: input.claimStatusPayer ?? null,
      isWritable: true,
    },
    systemProgram: { value: input.systemProgram ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Original args.
  const args = { ...input };

  // Resolve default values.
  if (!accounts.systemProgram.value) {
    accounts.systemProgram.value =
      '11111111111111111111111111111111' as Address<'11111111111111111111111111111111'>;
  }

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.epochState),
      getAccountMeta(accounts.vaultRegistry),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.claimStatusPayer),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeWeightTableInstructionDataEncoder().encode(
      args as InitializeWeightTableInstructionDataArgs
    ),
  } as InitializeWeightTableInstruction<
    TProgramAddress,
    TAccountEpochState,
    TAccountVaultRegistry,
    TAccountNcn,
    TAccountWeightTable,
    TAccountClaimStatusPayer,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeWeightTableInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    epochState: TAccountMetas[0];
    vaultRegistry: TAccountMetas[1];
    ncn: TAccountMetas[2];
    weightTable: TAccountMetas[3];
    claimStatusPayer: TAccountMetas[4];
    systemProgram: TAccountMetas[5];
  };
  data: InitializeWeightTableInstructionData;
};

export function parseInitializeWeightTableInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeWeightTableInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 6) {
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
      epochState: getNextAccount(),
      vaultRegistry: getNextAccount(),
      ncn: getNextAccount(),
      weightTable: getNextAccount(),
      claimStatusPayer: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeWeightTableInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

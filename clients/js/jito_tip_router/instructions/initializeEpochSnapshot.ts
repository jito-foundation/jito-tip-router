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

export const INITIALIZE_EPOCH_SNAPSHOT_DISCRIMINATOR = 7;

export function getInitializeEpochSnapshotDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_EPOCH_SNAPSHOT_DISCRIMINATOR);
}

export type InitializeEpochSnapshotInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountEpochSnapshot extends string | IAccountMeta<string> = string,
  TAccountPayer extends string | IAccountMeta<string> = string,
  TAccountRestakingProgram extends string | IAccountMeta<string> = string,
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
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? ReadonlyAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountEpochSnapshot extends string
        ? WritableAccount<TAccountEpochSnapshot>
        : TAccountEpochSnapshot,
      TAccountPayer extends string
        ? WritableSignerAccount<TAccountPayer> &
            IAccountSignerMeta<TAccountPayer>
        : TAccountPayer,
      TAccountRestakingProgram extends string
        ? ReadonlyAccount<TAccountRestakingProgram>
        : TAccountRestakingProgram,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type InitializeEpochSnapshotInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type InitializeEpochSnapshotInstructionDataArgs = {
  epoch: number | bigint;
};

export function getInitializeEpochSnapshotInstructionDataEncoder(): Encoder<InitializeEpochSnapshotInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_EPOCH_SNAPSHOT_DISCRIMINATOR,
    })
  );
}

export function getInitializeEpochSnapshotInstructionDataDecoder(): Decoder<InitializeEpochSnapshotInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getInitializeEpochSnapshotInstructionDataCodec(): Codec<
  InitializeEpochSnapshotInstructionDataArgs,
  InitializeEpochSnapshotInstructionData
> {
  return combineCodec(
    getInitializeEpochSnapshotInstructionDataEncoder(),
    getInitializeEpochSnapshotInstructionDataDecoder()
  );
}

export type InitializeEpochSnapshotInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountEpochSnapshot extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  epochSnapshot: Address<TAccountEpochSnapshot>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: InitializeEpochSnapshotInstructionDataArgs['epoch'];
};

export function getInitializeEpochSnapshotInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountEpochSnapshot extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeEpochSnapshotInput<
    TAccountConfig,
    TAccountNcn,
    TAccountWeightTable,
    TAccountEpochSnapshot,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeEpochSnapshotInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountWeightTable,
  TAccountEpochSnapshot,
  TAccountPayer,
  TAccountRestakingProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: false },
    epochSnapshot: { value: input.epochSnapshot ?? null, isWritable: true },
    payer: { value: input.payer ?? null, isWritable: true },
    restakingProgram: {
      value: input.restakingProgram ?? null,
      isWritable: false,
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
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.epochSnapshot),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeEpochSnapshotInstructionDataEncoder().encode(
      args as InitializeEpochSnapshotInstructionDataArgs
    ),
  } as InitializeEpochSnapshotInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountWeightTable,
    TAccountEpochSnapshot,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeEpochSnapshotInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    weightTable: TAccountMetas[2];
    epochSnapshot: TAccountMetas[3];
    payer: TAccountMetas[4];
    restakingProgram: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
  };
  data: InitializeEpochSnapshotInstructionData;
};

export function parseInitializeEpochSnapshotInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeEpochSnapshotInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 7) {
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
      ncn: getNextAccount(),
      weightTable: getNextAccount(),
      epochSnapshot: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeEpochSnapshotInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

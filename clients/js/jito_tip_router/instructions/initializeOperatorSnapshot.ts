/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getOptionDecoder,
  getOptionEncoder,
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
  type Option,
  type OptionOrNullable,
  type ReadonlyAccount,
  type TransactionSigner,
  type WritableAccount,
  type WritableSignerAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const INITIALIZE_OPERATOR_SNAPSHOT_DISCRIMINATOR = 6;

export function getInitializeOperatorSnapshotDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_OPERATOR_SNAPSHOT_DISCRIMINATOR);
}

export type InitializeOperatorSnapshotInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountNcnConfig extends string | IAccountMeta<string> = string,
  TAccountRestakingConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountOperator extends string | IAccountMeta<string> = string,
  TAccountNcnOperatorState extends string | IAccountMeta<string> = string,
  TAccountEpochSnapshot extends string | IAccountMeta<string> = string,
  TAccountOperatorSnapshot extends string | IAccountMeta<string> = string,
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
      TAccountNcnConfig extends string
        ? ReadonlyAccount<TAccountNcnConfig>
        : TAccountNcnConfig,
      TAccountRestakingConfig extends string
        ? ReadonlyAccount<TAccountRestakingConfig>
        : TAccountRestakingConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountOperator extends string
        ? ReadonlyAccount<TAccountOperator>
        : TAccountOperator,
      TAccountNcnOperatorState extends string
        ? ReadonlyAccount<TAccountNcnOperatorState>
        : TAccountNcnOperatorState,
      TAccountEpochSnapshot extends string
        ? WritableAccount<TAccountEpochSnapshot>
        : TAccountEpochSnapshot,
      TAccountOperatorSnapshot extends string
        ? WritableAccount<TAccountOperatorSnapshot>
        : TAccountOperatorSnapshot,
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

export type InitializeOperatorSnapshotInstructionData = {
  discriminator: number;
  firstSlotOfNcnEpoch: Option<bigint>;
};

export type InitializeOperatorSnapshotInstructionDataArgs = {
  firstSlotOfNcnEpoch: OptionOrNullable<number | bigint>;
};

export function getInitializeOperatorSnapshotInstructionDataEncoder(): Encoder<InitializeOperatorSnapshotInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['firstSlotOfNcnEpoch', getOptionEncoder(getU64Encoder())],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_OPERATOR_SNAPSHOT_DISCRIMINATOR,
    })
  );
}

export function getInitializeOperatorSnapshotInstructionDataDecoder(): Decoder<InitializeOperatorSnapshotInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['firstSlotOfNcnEpoch', getOptionDecoder(getU64Decoder())],
  ]);
}

export function getInitializeOperatorSnapshotInstructionDataCodec(): Codec<
  InitializeOperatorSnapshotInstructionDataArgs,
  InitializeOperatorSnapshotInstructionData
> {
  return combineCodec(
    getInitializeOperatorSnapshotInstructionDataEncoder(),
    getInitializeOperatorSnapshotInstructionDataDecoder()
  );
}

export type InitializeOperatorSnapshotInput<
  TAccountNcnConfig extends string = string,
  TAccountRestakingConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountOperator extends string = string,
  TAccountNcnOperatorState extends string = string,
  TAccountEpochSnapshot extends string = string,
  TAccountOperatorSnapshot extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  ncnConfig: Address<TAccountNcnConfig>;
  restakingConfig: Address<TAccountRestakingConfig>;
  ncn: Address<TAccountNcn>;
  operator: Address<TAccountOperator>;
  ncnOperatorState: Address<TAccountNcnOperatorState>;
  epochSnapshot: Address<TAccountEpochSnapshot>;
  operatorSnapshot: Address<TAccountOperatorSnapshot>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  firstSlotOfNcnEpoch: InitializeOperatorSnapshotInstructionDataArgs['firstSlotOfNcnEpoch'];
};

export function getInitializeOperatorSnapshotInstruction<
  TAccountNcnConfig extends string,
  TAccountRestakingConfig extends string,
  TAccountNcn extends string,
  TAccountOperator extends string,
  TAccountNcnOperatorState extends string,
  TAccountEpochSnapshot extends string,
  TAccountOperatorSnapshot extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeOperatorSnapshotInput<
    TAccountNcnConfig,
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountOperator,
    TAccountNcnOperatorState,
    TAccountEpochSnapshot,
    TAccountOperatorSnapshot,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeOperatorSnapshotInstruction<
  TProgramAddress,
  TAccountNcnConfig,
  TAccountRestakingConfig,
  TAccountNcn,
  TAccountOperator,
  TAccountNcnOperatorState,
  TAccountEpochSnapshot,
  TAccountOperatorSnapshot,
  TAccountPayer,
  TAccountRestakingProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    ncnConfig: { value: input.ncnConfig ?? null, isWritable: false },
    restakingConfig: {
      value: input.restakingConfig ?? null,
      isWritable: false,
    },
    ncn: { value: input.ncn ?? null, isWritable: false },
    operator: { value: input.operator ?? null, isWritable: false },
    ncnOperatorState: {
      value: input.ncnOperatorState ?? null,
      isWritable: false,
    },
    epochSnapshot: { value: input.epochSnapshot ?? null, isWritable: true },
    operatorSnapshot: {
      value: input.operatorSnapshot ?? null,
      isWritable: true,
    },
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
      getAccountMeta(accounts.ncnConfig),
      getAccountMeta(accounts.restakingConfig),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.operator),
      getAccountMeta(accounts.ncnOperatorState),
      getAccountMeta(accounts.epochSnapshot),
      getAccountMeta(accounts.operatorSnapshot),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeOperatorSnapshotInstructionDataEncoder().encode(
      args as InitializeOperatorSnapshotInstructionDataArgs
    ),
  } as InitializeOperatorSnapshotInstruction<
    TProgramAddress,
    TAccountNcnConfig,
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountOperator,
    TAccountNcnOperatorState,
    TAccountEpochSnapshot,
    TAccountOperatorSnapshot,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeOperatorSnapshotInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    ncnConfig: TAccountMetas[0];
    restakingConfig: TAccountMetas[1];
    ncn: TAccountMetas[2];
    operator: TAccountMetas[3];
    ncnOperatorState: TAccountMetas[4];
    epochSnapshot: TAccountMetas[5];
    operatorSnapshot: TAccountMetas[6];
    payer: TAccountMetas[7];
    restakingProgram: TAccountMetas[8];
    systemProgram: TAccountMetas[9];
  };
  data: InitializeOperatorSnapshotInstructionData;
};

export function parseInitializeOperatorSnapshotInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeOperatorSnapshotInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 10) {
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
      ncnConfig: getNextAccount(),
      restakingConfig: getNextAccount(),
      ncn: getNextAccount(),
      operator: getNextAccount(),
      ncnOperatorState: getNextAccount(),
      epochSnapshot: getNextAccount(),
      operatorSnapshot: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeOperatorSnapshotInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

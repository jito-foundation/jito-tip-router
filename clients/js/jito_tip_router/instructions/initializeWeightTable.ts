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

export const INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR = 3;

export function getInitializeWeightTableDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_WEIGHT_TABLE_DISCRIMINATOR);
}

export type InitializeWeightTableInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountRestakingConfig extends string | IAccountMeta<string> = string,
  TAccountNcnConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
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
      TAccountRestakingConfig extends string
        ? ReadonlyAccount<TAccountRestakingConfig>
        : TAccountRestakingConfig,
      TAccountNcnConfig extends string
        ? ReadonlyAccount<TAccountNcnConfig>
        : TAccountNcnConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
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

export type InitializeWeightTableInstructionData = {
  discriminator: number;
  firstSlotOfNcnEpoch: Option<bigint>;
};

export type InitializeWeightTableInstructionDataArgs = {
  firstSlotOfNcnEpoch: OptionOrNullable<number | bigint>;
};

export function getInitializeWeightTableInstructionDataEncoder(): Encoder<InitializeWeightTableInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['firstSlotOfNcnEpoch', getOptionEncoder(getU64Encoder())],
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
    ['firstSlotOfNcnEpoch', getOptionDecoder(getU64Decoder())],
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
  TAccountRestakingConfig extends string = string,
  TAccountNcnConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  restakingConfig: Address<TAccountRestakingConfig>;
  ncnConfig: Address<TAccountNcnConfig>;
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  firstSlotOfNcnEpoch: InitializeWeightTableInstructionDataArgs['firstSlotOfNcnEpoch'];
};

export function getInitializeWeightTableInstruction<
  TAccountRestakingConfig extends string,
  TAccountNcnConfig extends string,
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeWeightTableInput<
    TAccountRestakingConfig,
    TAccountNcnConfig,
    TAccountNcn,
    TAccountWeightTable,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeWeightTableInstruction<
  TProgramAddress,
  TAccountRestakingConfig,
  TAccountNcnConfig,
  TAccountNcn,
  TAccountWeightTable,
  TAccountPayer,
  TAccountRestakingProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    restakingConfig: {
      value: input.restakingConfig ?? null,
      isWritable: false,
    },
    ncnConfig: { value: input.ncnConfig ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
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
      getAccountMeta(accounts.restakingConfig),
      getAccountMeta(accounts.ncnConfig),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeWeightTableInstructionDataEncoder().encode(
      args as InitializeWeightTableInstructionDataArgs
    ),
  } as InitializeWeightTableInstruction<
    TProgramAddress,
    TAccountRestakingConfig,
    TAccountNcnConfig,
    TAccountNcn,
    TAccountWeightTable,
    TAccountPayer,
    TAccountRestakingProgram,
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
    restakingConfig: TAccountMetas[0];
    ncnConfig: TAccountMetas[1];
    ncn: TAccountMetas[2];
    weightTable: TAccountMetas[3];
    payer: TAccountMetas[4];
    restakingProgram: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
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
      restakingConfig: getNextAccount(),
      ncnConfig: getNextAccount(),
      ncn: getNextAccount(),
      weightTable: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeWeightTableInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

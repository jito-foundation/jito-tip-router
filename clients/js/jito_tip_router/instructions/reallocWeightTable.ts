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

export const REALLOC_WEIGHT_TABLE_DISCRIMINATOR = 27;

export function getReallocWeightTableDiscriminatorBytes() {
  return getU8Encoder().encode(REALLOC_WEIGHT_TABLE_DISCRIMINATOR);
}

export type ReallocWeightTableInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountNcnConfig extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountTrackedMints extends string | IAccountMeta<string> = string,
  TAccountPayer extends string | IAccountMeta<string> = string,
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
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountTrackedMints extends string
        ? ReadonlyAccount<TAccountTrackedMints>
        : TAccountTrackedMints,
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

export type ReallocWeightTableInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type ReallocWeightTableInstructionDataArgs = { epoch: number | bigint };

export function getReallocWeightTableInstructionDataEncoder(): Encoder<ReallocWeightTableInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({ ...value, discriminator: REALLOC_WEIGHT_TABLE_DISCRIMINATOR })
  );
}

export function getReallocWeightTableInstructionDataDecoder(): Decoder<ReallocWeightTableInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getReallocWeightTableInstructionDataCodec(): Codec<
  ReallocWeightTableInstructionDataArgs,
  ReallocWeightTableInstructionData
> {
  return combineCodec(
    getReallocWeightTableInstructionDataEncoder(),
    getReallocWeightTableInstructionDataDecoder()
  );
}

export type ReallocWeightTableInput<
  TAccountNcnConfig extends string = string,
  TAccountWeightTable extends string = string,
  TAccountNcn extends string = string,
  TAccountTrackedMints extends string = string,
  TAccountPayer extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  ncnConfig: Address<TAccountNcnConfig>;
  weightTable: Address<TAccountWeightTable>;
  ncn: Address<TAccountNcn>;
  trackedMints: Address<TAccountTrackedMints>;
  payer: TransactionSigner<TAccountPayer>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: ReallocWeightTableInstructionDataArgs['epoch'];
};

export function getReallocWeightTableInstruction<
  TAccountNcnConfig extends string,
  TAccountWeightTable extends string,
  TAccountNcn extends string,
  TAccountTrackedMints extends string,
  TAccountPayer extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: ReallocWeightTableInput<
    TAccountNcnConfig,
    TAccountWeightTable,
    TAccountNcn,
    TAccountTrackedMints,
    TAccountPayer,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): ReallocWeightTableInstruction<
  TProgramAddress,
  TAccountNcnConfig,
  TAccountWeightTable,
  TAccountNcn,
  TAccountTrackedMints,
  TAccountPayer,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    ncnConfig: { value: input.ncnConfig ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    trackedMints: { value: input.trackedMints ?? null, isWritable: false },
    payer: { value: input.payer ?? null, isWritable: true },
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
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.trackedMints),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getReallocWeightTableInstructionDataEncoder().encode(
      args as ReallocWeightTableInstructionDataArgs
    ),
  } as ReallocWeightTableInstruction<
    TProgramAddress,
    TAccountNcnConfig,
    TAccountWeightTable,
    TAccountNcn,
    TAccountTrackedMints,
    TAccountPayer,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedReallocWeightTableInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    ncnConfig: TAccountMetas[0];
    weightTable: TAccountMetas[1];
    ncn: TAccountMetas[2];
    trackedMints: TAccountMetas[3];
    payer: TAccountMetas[4];
    systemProgram: TAccountMetas[5];
  };
  data: ReallocWeightTableInstructionData;
};

export function parseReallocWeightTableInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedReallocWeightTableInstruction<TProgram, TAccountMetas> {
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
      ncnConfig: getNextAccount(),
      weightTable: getNextAccount(),
      ncn: getNextAccount(),
      trackedMints: getNextAccount(),
      payer: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getReallocWeightTableInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

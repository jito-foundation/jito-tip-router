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

export const SET_JTO_WEIGHT_DISCRIMINATOR = 7;

export function getSetJtoWeightDiscriminatorBytes() {
  return getU8Encoder().encode(SET_JTO_WEIGHT_DISCRIMINATOR);
}

export type SetJtoWeightInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountJtoUsdFeed extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountJtoUsdFeed extends string
        ? ReadonlyAccount<TAccountJtoUsdFeed>
        : TAccountJtoUsdFeed,
      ...TRemainingAccounts,
    ]
  >;

export type SetJtoWeightInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type SetJtoWeightInstructionDataArgs = { epoch: number | bigint };

export function getSetJtoWeightInstructionDataEncoder(): Encoder<SetJtoWeightInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({ ...value, discriminator: SET_JTO_WEIGHT_DISCRIMINATOR })
  );
}

export function getSetJtoWeightInstructionDataDecoder(): Decoder<SetJtoWeightInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getSetJtoWeightInstructionDataCodec(): Codec<
  SetJtoWeightInstructionDataArgs,
  SetJtoWeightInstructionData
> {
  return combineCodec(
    getSetJtoWeightInstructionDataEncoder(),
    getSetJtoWeightInstructionDataDecoder()
  );
}

export type SetJtoWeightInput<
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountJtoUsdFeed extends string = string,
> = {
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  jtoUsdFeed: Address<TAccountJtoUsdFeed>;
  epoch: SetJtoWeightInstructionDataArgs['epoch'];
};

export function getSetJtoWeightInstruction<
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountJtoUsdFeed extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: SetJtoWeightInput<
    TAccountNcn,
    TAccountWeightTable,
    TAccountJtoUsdFeed
  >,
  config?: { programAddress?: TProgramAddress }
): SetJtoWeightInstruction<
  TProgramAddress,
  TAccountNcn,
  TAccountWeightTable,
  TAccountJtoUsdFeed
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
    jtoUsdFeed: { value: input.jtoUsdFeed ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Original args.
  const args = { ...input };

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.weightTable),
      getAccountMeta(accounts.jtoUsdFeed),
    ],
    programAddress,
    data: getSetJtoWeightInstructionDataEncoder().encode(
      args as SetJtoWeightInstructionDataArgs
    ),
  } as SetJtoWeightInstruction<
    TProgramAddress,
    TAccountNcn,
    TAccountWeightTable,
    TAccountJtoUsdFeed
  >;

  return instruction;
}

export type ParsedSetJtoWeightInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    ncn: TAccountMetas[0];
    weightTable: TAccountMetas[1];
    jtoUsdFeed: TAccountMetas[2];
  };
  data: SetJtoWeightInstructionData;
};

export function parseSetJtoWeightInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedSetJtoWeightInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 3) {
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
      ncn: getNextAccount(),
      weightTable: getNextAccount(),
      jtoUsdFeed: getNextAccount(),
    },
    data: getSetJtoWeightInstructionDataDecoder().decode(instruction.data),
  };
}
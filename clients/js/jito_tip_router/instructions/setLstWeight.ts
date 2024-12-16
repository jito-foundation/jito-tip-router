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

export const SET_LST_WEIGHT_DISCRIMINATOR = 6;

export function getSetLstWeightDiscriminatorBytes() {
  return getU8Encoder().encode(SET_LST_WEIGHT_DISCRIMINATOR);
}

export type SetLstWeightInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountWeightTable extends string | IAccountMeta<string> = string,
  TAccountMint extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountWeightTable extends string
        ? WritableAccount<TAccountWeightTable>
        : TAccountWeightTable,
      TAccountMint extends string
        ? ReadonlyAccount<TAccountMint>
        : TAccountMint,
      ...TRemainingAccounts,
    ]
  >;

export type SetLstWeightInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type SetLstWeightInstructionDataArgs = { epoch: number | bigint };

export function getSetLstWeightInstructionDataEncoder(): Encoder<SetLstWeightInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({ ...value, discriminator: SET_LST_WEIGHT_DISCRIMINATOR })
  );
}

export function getSetLstWeightInstructionDataDecoder(): Decoder<SetLstWeightInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getSetLstWeightInstructionDataCodec(): Codec<
  SetLstWeightInstructionDataArgs,
  SetLstWeightInstructionData
> {
  return combineCodec(
    getSetLstWeightInstructionDataEncoder(),
    getSetLstWeightInstructionDataDecoder()
  );
}

export type SetLstWeightInput<
  TAccountNcn extends string = string,
  TAccountWeightTable extends string = string,
  TAccountMint extends string = string,
> = {
  ncn: Address<TAccountNcn>;
  weightTable: Address<TAccountWeightTable>;
  mint: Address<TAccountMint>;
  epoch: SetLstWeightInstructionDataArgs['epoch'];
};

export function getSetLstWeightInstruction<
  TAccountNcn extends string,
  TAccountWeightTable extends string,
  TAccountMint extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: SetLstWeightInput<TAccountNcn, TAccountWeightTable, TAccountMint>,
  config?: { programAddress?: TProgramAddress }
): SetLstWeightInstruction<
  TProgramAddress,
  TAccountNcn,
  TAccountWeightTable,
  TAccountMint
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    ncn: { value: input.ncn ?? null, isWritable: false },
    weightTable: { value: input.weightTable ?? null, isWritable: true },
    mint: { value: input.mint ?? null, isWritable: false },
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
      getAccountMeta(accounts.mint),
    ],
    programAddress,
    data: getSetLstWeightInstructionDataEncoder().encode(
      args as SetLstWeightInstructionDataArgs
    ),
  } as SetLstWeightInstruction<
    TProgramAddress,
    TAccountNcn,
    TAccountWeightTable,
    TAccountMint
  >;

  return instruction;
}

export type ParsedSetLstWeightInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    ncn: TAccountMetas[0];
    weightTable: TAccountMetas[1];
    mint: TAccountMetas[2];
  };
  data: SetLstWeightInstructionData;
};

export function parseSetLstWeightInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedSetLstWeightInstruction<TProgram, TAccountMetas> {
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
      mint: getNextAccount(),
    },
    data: getSetLstWeightInstructionDataDecoder().decode(instruction.data),
  };
}
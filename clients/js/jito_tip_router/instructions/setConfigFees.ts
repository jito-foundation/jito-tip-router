/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getAddressDecoder,
  getAddressEncoder,
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
  type ReadonlySignerAccount,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const SET_CONFIG_FEES_DISCRIMINATOR = 4;

export function getSetConfigFeesDiscriminatorBytes() {
  return getU8Encoder().encode(SET_CONFIG_FEES_DISCRIMINATOR);
}

export type SetConfigFeesInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountNcnAdmin extends string | IAccountMeta<string> = string,
  TAccountRestakingProgramId extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountConfig extends string
        ? WritableAccount<TAccountConfig>
        : TAccountConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountNcnAdmin extends string
        ? ReadonlySignerAccount<TAccountNcnAdmin> &
            IAccountSignerMeta<TAccountNcnAdmin>
        : TAccountNcnAdmin,
      TAccountRestakingProgramId extends string
        ? ReadonlyAccount<TAccountRestakingProgramId>
        : TAccountRestakingProgramId,
      ...TRemainingAccounts,
    ]
  >;

export type SetConfigFeesInstructionData = {
  discriminator: number;
  newDaoFeeBps: Option<bigint>;
  newNcnFeeBps: Option<bigint>;
  newBlockEngineFeeBps: Option<bigint>;
  newFeeWallet: Option<Address>;
};

export type SetConfigFeesInstructionDataArgs = {
  newDaoFeeBps: OptionOrNullable<number | bigint>;
  newNcnFeeBps: OptionOrNullable<number | bigint>;
  newBlockEngineFeeBps: OptionOrNullable<number | bigint>;
  newFeeWallet: OptionOrNullable<Address>;
};

export function getSetConfigFeesInstructionDataEncoder(): Encoder<SetConfigFeesInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['newDaoFeeBps', getOptionEncoder(getU64Encoder())],
      ['newNcnFeeBps', getOptionEncoder(getU64Encoder())],
      ['newBlockEngineFeeBps', getOptionEncoder(getU64Encoder())],
      ['newFeeWallet', getOptionEncoder(getAddressEncoder())],
    ]),
    (value) => ({ ...value, discriminator: SET_CONFIG_FEES_DISCRIMINATOR })
  );
}

export function getSetConfigFeesInstructionDataDecoder(): Decoder<SetConfigFeesInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['newDaoFeeBps', getOptionDecoder(getU64Decoder())],
    ['newNcnFeeBps', getOptionDecoder(getU64Decoder())],
    ['newBlockEngineFeeBps', getOptionDecoder(getU64Decoder())],
    ['newFeeWallet', getOptionDecoder(getAddressDecoder())],
  ]);
}

export function getSetConfigFeesInstructionDataCodec(): Codec<
  SetConfigFeesInstructionDataArgs,
  SetConfigFeesInstructionData
> {
  return combineCodec(
    getSetConfigFeesInstructionDataEncoder(),
    getSetConfigFeesInstructionDataDecoder()
  );
}

export type SetConfigFeesInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountNcnAdmin extends string = string,
  TAccountRestakingProgramId extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  ncnAdmin: TransactionSigner<TAccountNcnAdmin>;
  restakingProgramId: Address<TAccountRestakingProgramId>;
  newDaoFeeBps: SetConfigFeesInstructionDataArgs['newDaoFeeBps'];
  newNcnFeeBps: SetConfigFeesInstructionDataArgs['newNcnFeeBps'];
  newBlockEngineFeeBps: SetConfigFeesInstructionDataArgs['newBlockEngineFeeBps'];
  newFeeWallet: SetConfigFeesInstructionDataArgs['newFeeWallet'];
};

export function getSetConfigFeesInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountNcnAdmin extends string,
  TAccountRestakingProgramId extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: SetConfigFeesInput<
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin,
    TAccountRestakingProgramId
  >,
  config?: { programAddress?: TProgramAddress }
): SetConfigFeesInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountNcnAdmin,
  TAccountRestakingProgramId
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    ncnAdmin: { value: input.ncnAdmin ?? null, isWritable: false },
    restakingProgramId: {
      value: input.restakingProgramId ?? null,
      isWritable: false,
    },
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
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.ncnAdmin),
      getAccountMeta(accounts.restakingProgramId),
    ],
    programAddress,
    data: getSetConfigFeesInstructionDataEncoder().encode(
      args as SetConfigFeesInstructionDataArgs
    ),
  } as SetConfigFeesInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin,
    TAccountRestakingProgramId
  >;

  return instruction;
}

export type ParsedSetConfigFeesInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    ncnAdmin: TAccountMetas[2];
    restakingProgramId: TAccountMetas[3];
  };
  data: SetConfigFeesInstructionData;
};

export function parseSetConfigFeesInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedSetConfigFeesInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 4) {
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
      ncnAdmin: getNextAccount(),
      restakingProgramId: getNextAccount(),
    },
    data: getSetConfigFeesInstructionDataDecoder().decode(instruction.data),
  };
}

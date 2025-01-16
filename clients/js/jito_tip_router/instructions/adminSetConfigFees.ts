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
  getU16Decoder,
  getU16Encoder,
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

export const ADMIN_SET_CONFIG_FEES_DISCRIMINATOR = 28;

export function getAdminSetConfigFeesDiscriminatorBytes() {
  return getU8Encoder().encode(ADMIN_SET_CONFIG_FEES_DISCRIMINATOR);
}

export type AdminSetConfigFeesInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountNcnAdmin extends string | IAccountMeta<string> = string,
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
      ...TRemainingAccounts,
    ]
  >;

export type AdminSetConfigFeesInstructionData = {
  discriminator: number;
  newBlockEngineFeeBps: Option<number>;
  baseFeeGroup: Option<number>;
  newBaseFeeWallet: Option<Address>;
  newBaseFeeBps: Option<number>;
  ncnFeeGroup: Option<number>;
  newNcnFeeBps: Option<number>;
};

export type AdminSetConfigFeesInstructionDataArgs = {
  newBlockEngineFeeBps: OptionOrNullable<number>;
  baseFeeGroup: OptionOrNullable<number>;
  newBaseFeeWallet: OptionOrNullable<Address>;
  newBaseFeeBps: OptionOrNullable<number>;
  ncnFeeGroup: OptionOrNullable<number>;
  newNcnFeeBps: OptionOrNullable<number>;
};

export function getAdminSetConfigFeesInstructionDataEncoder(): Encoder<AdminSetConfigFeesInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['newBlockEngineFeeBps', getOptionEncoder(getU16Encoder())],
      ['baseFeeGroup', getOptionEncoder(getU8Encoder())],
      ['newBaseFeeWallet', getOptionEncoder(getAddressEncoder())],
      ['newBaseFeeBps', getOptionEncoder(getU16Encoder())],
      ['ncnFeeGroup', getOptionEncoder(getU8Encoder())],
      ['newNcnFeeBps', getOptionEncoder(getU16Encoder())],
    ]),
    (value) => ({
      ...value,
      discriminator: ADMIN_SET_CONFIG_FEES_DISCRIMINATOR,
    })
  );
}

export function getAdminSetConfigFeesInstructionDataDecoder(): Decoder<AdminSetConfigFeesInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['newBlockEngineFeeBps', getOptionDecoder(getU16Decoder())],
    ['baseFeeGroup', getOptionDecoder(getU8Decoder())],
    ['newBaseFeeWallet', getOptionDecoder(getAddressDecoder())],
    ['newBaseFeeBps', getOptionDecoder(getU16Decoder())],
    ['ncnFeeGroup', getOptionDecoder(getU8Decoder())],
    ['newNcnFeeBps', getOptionDecoder(getU16Decoder())],
  ]);
}

export function getAdminSetConfigFeesInstructionDataCodec(): Codec<
  AdminSetConfigFeesInstructionDataArgs,
  AdminSetConfigFeesInstructionData
> {
  return combineCodec(
    getAdminSetConfigFeesInstructionDataEncoder(),
    getAdminSetConfigFeesInstructionDataDecoder()
  );
}

export type AdminSetConfigFeesInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountNcnAdmin extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  ncnAdmin: TransactionSigner<TAccountNcnAdmin>;
  newBlockEngineFeeBps: AdminSetConfigFeesInstructionDataArgs['newBlockEngineFeeBps'];
  baseFeeGroup: AdminSetConfigFeesInstructionDataArgs['baseFeeGroup'];
  newBaseFeeWallet: AdminSetConfigFeesInstructionDataArgs['newBaseFeeWallet'];
  newBaseFeeBps: AdminSetConfigFeesInstructionDataArgs['newBaseFeeBps'];
  ncnFeeGroup: AdminSetConfigFeesInstructionDataArgs['ncnFeeGroup'];
  newNcnFeeBps: AdminSetConfigFeesInstructionDataArgs['newNcnFeeBps'];
};

export function getAdminSetConfigFeesInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountNcnAdmin extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: AdminSetConfigFeesInput<TAccountConfig, TAccountNcn, TAccountNcnAdmin>,
  config?: { programAddress?: TProgramAddress }
): AdminSetConfigFeesInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountNcnAdmin
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    ncnAdmin: { value: input.ncnAdmin ?? null, isWritable: false },
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
    ],
    programAddress,
    data: getAdminSetConfigFeesInstructionDataEncoder().encode(
      args as AdminSetConfigFeesInstructionDataArgs
    ),
  } as AdminSetConfigFeesInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin
  >;

  return instruction;
}

export type ParsedAdminSetConfigFeesInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    ncnAdmin: TAccountMetas[2];
  };
  data: AdminSetConfigFeesInstructionData;
};

export function parseAdminSetConfigFeesInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedAdminSetConfigFeesInstruction<TProgram, TAccountMetas> {
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
      config: getNextAccount(),
      ncn: getNextAccount(),
      ncnAdmin: getNextAccount(),
    },
    data: getAdminSetConfigFeesInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

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
  type ReadonlySignerAccount,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';
import {
  getConfigAdminRoleDecoder,
  getConfigAdminRoleEncoder,
  type ConfigAdminRole,
  type ConfigAdminRoleArgs,
} from '../types';

export const ADMIN_SET_NEW_ADMIN_DISCRIMINATOR = 29;

export function getAdminSetNewAdminDiscriminatorBytes() {
  return getU8Encoder().encode(ADMIN_SET_NEW_ADMIN_DISCRIMINATOR);
}

export type AdminSetNewAdminInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountNcnAdmin extends string | IAccountMeta<string> = string,
  TAccountNewAdmin extends string | IAccountMeta<string> = string,
  TAccountRestakingProgram extends string | IAccountMeta<string> = string,
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
      TAccountNewAdmin extends string
        ? ReadonlyAccount<TAccountNewAdmin>
        : TAccountNewAdmin,
      TAccountRestakingProgram extends string
        ? ReadonlyAccount<TAccountRestakingProgram>
        : TAccountRestakingProgram,
      ...TRemainingAccounts,
    ]
  >;

export type AdminSetNewAdminInstructionData = {
  discriminator: number;
  role: ConfigAdminRole;
};

export type AdminSetNewAdminInstructionDataArgs = { role: ConfigAdminRoleArgs };

export function getAdminSetNewAdminInstructionDataEncoder(): Encoder<AdminSetNewAdminInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['role', getConfigAdminRoleEncoder()],
    ]),
    (value) => ({ ...value, discriminator: ADMIN_SET_NEW_ADMIN_DISCRIMINATOR })
  );
}

export function getAdminSetNewAdminInstructionDataDecoder(): Decoder<AdminSetNewAdminInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['role', getConfigAdminRoleDecoder()],
  ]);
}

export function getAdminSetNewAdminInstructionDataCodec(): Codec<
  AdminSetNewAdminInstructionDataArgs,
  AdminSetNewAdminInstructionData
> {
  return combineCodec(
    getAdminSetNewAdminInstructionDataEncoder(),
    getAdminSetNewAdminInstructionDataDecoder()
  );
}

export type AdminSetNewAdminInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountNcnAdmin extends string = string,
  TAccountNewAdmin extends string = string,
  TAccountRestakingProgram extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  ncnAdmin: TransactionSigner<TAccountNcnAdmin>;
  newAdmin: Address<TAccountNewAdmin>;
  restakingProgram: Address<TAccountRestakingProgram>;
  role: AdminSetNewAdminInstructionDataArgs['role'];
};

export function getAdminSetNewAdminInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountNcnAdmin extends string,
  TAccountNewAdmin extends string,
  TAccountRestakingProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: AdminSetNewAdminInput<
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin,
    TAccountNewAdmin,
    TAccountRestakingProgram
  >,
  config?: { programAddress?: TProgramAddress }
): AdminSetNewAdminInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountNcnAdmin,
  TAccountNewAdmin,
  TAccountRestakingProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    ncnAdmin: { value: input.ncnAdmin ?? null, isWritable: false },
    newAdmin: { value: input.newAdmin ?? null, isWritable: false },
    restakingProgram: {
      value: input.restakingProgram ?? null,
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
      getAccountMeta(accounts.newAdmin),
      getAccountMeta(accounts.restakingProgram),
    ],
    programAddress,
    data: getAdminSetNewAdminInstructionDataEncoder().encode(
      args as AdminSetNewAdminInstructionDataArgs
    ),
  } as AdminSetNewAdminInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountNcnAdmin,
    TAccountNewAdmin,
    TAccountRestakingProgram
  >;

  return instruction;
}

export type ParsedAdminSetNewAdminInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    ncnAdmin: TAccountMetas[2];
    newAdmin: TAccountMetas[3];
    restakingProgram: TAccountMetas[4];
  };
  data: AdminSetNewAdminInstructionData;
};

export function parseAdminSetNewAdminInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedAdminSetNewAdminInstruction<TProgram, TAccountMetas> {
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
      ncn: getNextAccount(),
      ncnAdmin: getNextAccount(),
      newAdmin: getNextAccount(),
      restakingProgram: getNextAccount(),
    },
    data: getAdminSetNewAdminInstructionDataDecoder().decode(instruction.data),
  };
}

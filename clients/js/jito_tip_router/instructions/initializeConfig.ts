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
  getU16Decoder,
  getU16Encoder,
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
  type ReadonlySignerAccount,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const INITIALIZE_CONFIG_DISCRIMINATOR = 0;

export function getInitializeConfigDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_CONFIG_DISCRIMINATOR);
}

export type InitializeConfigInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountFeeWallet extends string | IAccountMeta<string> = string,
  TAccountNcnAdmin extends string | IAccountMeta<string> = string,
  TAccountTieBreakerAdmin extends string | IAccountMeta<string> = string,
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
        ? WritableAccount<TAccountConfig>
        : TAccountConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountFeeWallet extends string
        ? ReadonlyAccount<TAccountFeeWallet>
        : TAccountFeeWallet,
      TAccountNcnAdmin extends string
        ? ReadonlySignerAccount<TAccountNcnAdmin> &
            IAccountSignerMeta<TAccountNcnAdmin>
        : TAccountNcnAdmin,
      TAccountTieBreakerAdmin extends string
        ? ReadonlyAccount<TAccountTieBreakerAdmin>
        : TAccountTieBreakerAdmin,
      TAccountRestakingProgram extends string
        ? ReadonlyAccount<TAccountRestakingProgram>
        : TAccountRestakingProgram,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type InitializeConfigInstructionData = {
  discriminator: number;
  blockEngineFeeBps: number;
  daoFeeBps: number;
  defaultNcnFeeBps: number;
  epochsBeforeStall: bigint;
  validSlotsAfterConsensus: bigint;
};

export type InitializeConfigInstructionDataArgs = {
  blockEngineFeeBps: number;
  daoFeeBps: number;
  defaultNcnFeeBps: number;
  epochsBeforeStall: number | bigint;
  validSlotsAfterConsensus: number | bigint;
};

export function getInitializeConfigInstructionDataEncoder(): Encoder<InitializeConfigInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['blockEngineFeeBps', getU16Encoder()],
      ['daoFeeBps', getU16Encoder()],
      ['defaultNcnFeeBps', getU16Encoder()],
      ['epochsBeforeStall', getU64Encoder()],
      ['validSlotsAfterConsensus', getU64Encoder()],
    ]),
    (value) => ({ ...value, discriminator: INITIALIZE_CONFIG_DISCRIMINATOR })
  );
}

export function getInitializeConfigInstructionDataDecoder(): Decoder<InitializeConfigInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['blockEngineFeeBps', getU16Decoder()],
    ['daoFeeBps', getU16Decoder()],
    ['defaultNcnFeeBps', getU16Decoder()],
    ['epochsBeforeStall', getU64Decoder()],
    ['validSlotsAfterConsensus', getU64Decoder()],
  ]);
}

export function getInitializeConfigInstructionDataCodec(): Codec<
  InitializeConfigInstructionDataArgs,
  InitializeConfigInstructionData
> {
  return combineCodec(
    getInitializeConfigInstructionDataEncoder(),
    getInitializeConfigInstructionDataDecoder()
  );
}

export type InitializeConfigInput<
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountFeeWallet extends string = string,
  TAccountNcnAdmin extends string = string,
  TAccountTieBreakerAdmin extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  feeWallet: Address<TAccountFeeWallet>;
  ncnAdmin: TransactionSigner<TAccountNcnAdmin>;
  tieBreakerAdmin: Address<TAccountTieBreakerAdmin>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  blockEngineFeeBps: InitializeConfigInstructionDataArgs['blockEngineFeeBps'];
  daoFeeBps: InitializeConfigInstructionDataArgs['daoFeeBps'];
  defaultNcnFeeBps: InitializeConfigInstructionDataArgs['defaultNcnFeeBps'];
  epochsBeforeStall: InitializeConfigInstructionDataArgs['epochsBeforeStall'];
  validSlotsAfterConsensus: InitializeConfigInstructionDataArgs['validSlotsAfterConsensus'];
};

export function getInitializeConfigInstruction<
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountFeeWallet extends string,
  TAccountNcnAdmin extends string,
  TAccountTieBreakerAdmin extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeConfigInput<
    TAccountConfig,
    TAccountNcn,
    TAccountFeeWallet,
    TAccountNcnAdmin,
    TAccountTieBreakerAdmin,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeConfigInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountNcn,
  TAccountFeeWallet,
  TAccountNcnAdmin,
  TAccountTieBreakerAdmin,
  TAccountRestakingProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    feeWallet: { value: input.feeWallet ?? null, isWritable: false },
    ncnAdmin: { value: input.ncnAdmin ?? null, isWritable: false },
    tieBreakerAdmin: {
      value: input.tieBreakerAdmin ?? null,
      isWritable: false,
    },
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
      getAccountMeta(accounts.feeWallet),
      getAccountMeta(accounts.ncnAdmin),
      getAccountMeta(accounts.tieBreakerAdmin),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeConfigInstructionDataEncoder().encode(
      args as InitializeConfigInstructionDataArgs
    ),
  } as InitializeConfigInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountNcn,
    TAccountFeeWallet,
    TAccountNcnAdmin,
    TAccountTieBreakerAdmin,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeConfigInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ncn: TAccountMetas[1];
    feeWallet: TAccountMetas[2];
    ncnAdmin: TAccountMetas[3];
    tieBreakerAdmin: TAccountMetas[4];
    restakingProgram: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
  };
  data: InitializeConfigInstructionData;
};

export function parseInitializeConfigInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeConfigInstruction<TProgram, TAccountMetas> {
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
      feeWallet: getNextAccount(),
      ncnAdmin: getNextAccount(),
      tieBreakerAdmin: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeConfigInstructionDataDecoder().decode(instruction.data),
  };
}

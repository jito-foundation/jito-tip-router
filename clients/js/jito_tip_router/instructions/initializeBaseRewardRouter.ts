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

export const INITIALIZE_BASE_REWARD_ROUTER_DISCRIMINATOR = 11;

export function getInitializeBaseRewardRouterDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_BASE_REWARD_ROUTER_DISCRIMINATOR);
}

export type InitializeBaseRewardRouterInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountRestakingConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountBaseRewardRouter extends string | IAccountMeta<string> = string,
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
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountBaseRewardRouter extends string
        ? WritableAccount<TAccountBaseRewardRouter>
        : TAccountBaseRewardRouter,
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

export type InitializeBaseRewardRouterInstructionData = {
  discriminator: number;
  epoch: bigint;
};

export type InitializeBaseRewardRouterInstructionDataArgs = {
  epoch: number | bigint;
};

export function getInitializeBaseRewardRouterInstructionDataEncoder(): Encoder<InitializeBaseRewardRouterInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_BASE_REWARD_ROUTER_DISCRIMINATOR,
    })
  );
}

export function getInitializeBaseRewardRouterInstructionDataDecoder(): Decoder<InitializeBaseRewardRouterInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getInitializeBaseRewardRouterInstructionDataCodec(): Codec<
  InitializeBaseRewardRouterInstructionDataArgs,
  InitializeBaseRewardRouterInstructionData
> {
  return combineCodec(
    getInitializeBaseRewardRouterInstructionDataEncoder(),
    getInitializeBaseRewardRouterInstructionDataDecoder()
  );
}

export type InitializeBaseRewardRouterInput<
  TAccountRestakingConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountBaseRewardRouter extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  restakingConfig: Address<TAccountRestakingConfig>;
  ncn: Address<TAccountNcn>;
  baseRewardRouter: Address<TAccountBaseRewardRouter>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: InitializeBaseRewardRouterInstructionDataArgs['epoch'];
};

export function getInitializeBaseRewardRouterInstruction<
  TAccountRestakingConfig extends string,
  TAccountNcn extends string,
  TAccountBaseRewardRouter extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeBaseRewardRouterInput<
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeBaseRewardRouterInstruction<
  TProgramAddress,
  TAccountRestakingConfig,
  TAccountNcn,
  TAccountBaseRewardRouter,
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
    ncn: { value: input.ncn ?? null, isWritable: false },
    baseRewardRouter: {
      value: input.baseRewardRouter ?? null,
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
      getAccountMeta(accounts.restakingConfig),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.baseRewardRouter),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeBaseRewardRouterInstructionDataEncoder().encode(
      args as InitializeBaseRewardRouterInstructionDataArgs
    ),
  } as InitializeBaseRewardRouterInstruction<
    TProgramAddress,
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeBaseRewardRouterInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    restakingConfig: TAccountMetas[0];
    ncn: TAccountMetas[1];
    baseRewardRouter: TAccountMetas[2];
    payer: TAccountMetas[3];
    restakingProgram: TAccountMetas[4];
    systemProgram: TAccountMetas[5];
  };
  data: InitializeBaseRewardRouterInstructionData;
};

export function parseInitializeBaseRewardRouterInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeBaseRewardRouterInstruction<TProgram, TAccountMetas> {
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
      restakingConfig: getNextAccount(),
      ncn: getNextAccount(),
      baseRewardRouter: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeBaseRewardRouterInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

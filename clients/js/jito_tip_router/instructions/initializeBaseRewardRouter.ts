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

export const INITIALIZE_BASE_REWARD_ROUTER_DISCRIMINATOR = 17;

export function getInitializeBaseRewardRouterDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_BASE_REWARD_ROUTER_DISCRIMINATOR);
}

export type InitializeBaseRewardRouterInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountEpochState extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountBaseRewardRouter extends string | IAccountMeta<string> = string,
  TAccountBaseRewardReceiver extends string | IAccountMeta<string> = string,
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
      TAccountEpochState extends string
        ? ReadonlyAccount<TAccountEpochState>
        : TAccountEpochState,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountBaseRewardRouter extends string
        ? WritableAccount<TAccountBaseRewardRouter>
        : TAccountBaseRewardRouter,
      TAccountBaseRewardReceiver extends string
        ? WritableAccount<TAccountBaseRewardReceiver>
        : TAccountBaseRewardReceiver,
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
  TAccountEpochState extends string = string,
  TAccountNcn extends string = string,
  TAccountBaseRewardRouter extends string = string,
  TAccountBaseRewardReceiver extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  epochState: Address<TAccountEpochState>;
  ncn: Address<TAccountNcn>;
  baseRewardRouter: Address<TAccountBaseRewardRouter>;
  baseRewardReceiver: Address<TAccountBaseRewardReceiver>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  epoch: InitializeBaseRewardRouterInstructionDataArgs['epoch'];
};

export function getInitializeBaseRewardRouterInstruction<
  TAccountEpochState extends string,
  TAccountNcn extends string,
  TAccountBaseRewardRouter extends string,
  TAccountBaseRewardReceiver extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeBaseRewardRouterInput<
    TAccountEpochState,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountBaseRewardReceiver,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeBaseRewardRouterInstruction<
  TProgramAddress,
  TAccountEpochState,
  TAccountNcn,
  TAccountBaseRewardRouter,
  TAccountBaseRewardReceiver,
  TAccountPayer,
  TAccountRestakingProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    epochState: { value: input.epochState ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    baseRewardRouter: {
      value: input.baseRewardRouter ?? null,
      isWritable: true,
    },
    baseRewardReceiver: {
      value: input.baseRewardReceiver ?? null,
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
      getAccountMeta(accounts.epochState),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.baseRewardRouter),
      getAccountMeta(accounts.baseRewardReceiver),
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
    TAccountEpochState,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountBaseRewardReceiver,
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
    epochState: TAccountMetas[0];
    ncn: TAccountMetas[1];
    baseRewardRouter: TAccountMetas[2];
    baseRewardReceiver: TAccountMetas[3];
    payer: TAccountMetas[4];
    restakingProgram: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
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
      epochState: getNextAccount(),
      ncn: getNextAccount(),
      baseRewardRouter: getNextAccount(),
      baseRewardReceiver: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeBaseRewardRouterInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

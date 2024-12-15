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

export const INITIALIZE_NCN_REWARD_ROUTER_DISCRIMINATOR = 13;

export function getInitializeNcnRewardRouterDiscriminatorBytes() {
  return getU8Encoder().encode(INITIALIZE_NCN_REWARD_ROUTER_DISCRIMINATOR);
}

export type InitializeNcnRewardRouterInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountRestakingConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountOperator extends string | IAccountMeta<string> = string,
  TAccountNcnRewardRouter extends string | IAccountMeta<string> = string,
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
      TAccountOperator extends string
        ? ReadonlyAccount<TAccountOperator>
        : TAccountOperator,
      TAccountNcnRewardRouter extends string
        ? WritableAccount<TAccountNcnRewardRouter>
        : TAccountNcnRewardRouter,
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

export type InitializeNcnRewardRouterInstructionData = {
  discriminator: number;
  ncnFeeGroup: number;
  epoch: bigint;
};

export type InitializeNcnRewardRouterInstructionDataArgs = {
  ncnFeeGroup: number;
  epoch: number | bigint;
};

export function getInitializeNcnRewardRouterInstructionDataEncoder(): Encoder<InitializeNcnRewardRouterInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['ncnFeeGroup', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: INITIALIZE_NCN_REWARD_ROUTER_DISCRIMINATOR,
    })
  );
}

export function getInitializeNcnRewardRouterInstructionDataDecoder(): Decoder<InitializeNcnRewardRouterInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['ncnFeeGroup', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getInitializeNcnRewardRouterInstructionDataCodec(): Codec<
  InitializeNcnRewardRouterInstructionDataArgs,
  InitializeNcnRewardRouterInstructionData
> {
  return combineCodec(
    getInitializeNcnRewardRouterInstructionDataEncoder(),
    getInitializeNcnRewardRouterInstructionDataDecoder()
  );
}

export type InitializeNcnRewardRouterInput<
  TAccountRestakingConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountOperator extends string = string,
  TAccountNcnRewardRouter extends string = string,
  TAccountPayer extends string = string,
  TAccountRestakingProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  restakingConfig: Address<TAccountRestakingConfig>;
  ncn: Address<TAccountNcn>;
  operator: Address<TAccountOperator>;
  ncnRewardRouter: Address<TAccountNcnRewardRouter>;
  payer: TransactionSigner<TAccountPayer>;
  restakingProgram: Address<TAccountRestakingProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  ncnFeeGroup: InitializeNcnRewardRouterInstructionDataArgs['ncnFeeGroup'];
  epoch: InitializeNcnRewardRouterInstructionDataArgs['epoch'];
};

export function getInitializeNcnRewardRouterInstruction<
  TAccountRestakingConfig extends string,
  TAccountNcn extends string,
  TAccountOperator extends string,
  TAccountNcnRewardRouter extends string,
  TAccountPayer extends string,
  TAccountRestakingProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: InitializeNcnRewardRouterInput<
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountOperator,
    TAccountNcnRewardRouter,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): InitializeNcnRewardRouterInstruction<
  TProgramAddress,
  TAccountRestakingConfig,
  TAccountNcn,
  TAccountOperator,
  TAccountNcnRewardRouter,
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
    operator: { value: input.operator ?? null, isWritable: false },
    ncnRewardRouter: { value: input.ncnRewardRouter ?? null, isWritable: true },
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
      getAccountMeta(accounts.operator),
      getAccountMeta(accounts.ncnRewardRouter),
      getAccountMeta(accounts.payer),
      getAccountMeta(accounts.restakingProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getInitializeNcnRewardRouterInstructionDataEncoder().encode(
      args as InitializeNcnRewardRouterInstructionDataArgs
    ),
  } as InitializeNcnRewardRouterInstruction<
    TProgramAddress,
    TAccountRestakingConfig,
    TAccountNcn,
    TAccountOperator,
    TAccountNcnRewardRouter,
    TAccountPayer,
    TAccountRestakingProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedInitializeNcnRewardRouterInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    restakingConfig: TAccountMetas[0];
    ncn: TAccountMetas[1];
    operator: TAccountMetas[2];
    ncnRewardRouter: TAccountMetas[3];
    payer: TAccountMetas[4];
    restakingProgram: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
  };
  data: InitializeNcnRewardRouterInstructionData;
};

export function parseInitializeNcnRewardRouterInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedInitializeNcnRewardRouterInstruction<TProgram, TAccountMetas> {
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
      ncn: getNextAccount(),
      operator: getNextAccount(),
      ncnRewardRouter: getNextAccount(),
      payer: getNextAccount(),
      restakingProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getInitializeNcnRewardRouterInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

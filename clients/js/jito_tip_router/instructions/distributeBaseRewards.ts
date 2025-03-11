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

export const DISTRIBUTE_BASE_REWARDS_DISCRIMINATOR = 23;

export function getDistributeBaseRewardsDiscriminatorBytes() {
  return getU8Encoder().encode(DISTRIBUTE_BASE_REWARDS_DISCRIMINATOR);
}

export type DistributeBaseRewardsInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountEpochState extends string | IAccountMeta<string> = string,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountBaseRewardRouter extends string | IAccountMeta<string> = string,
  TAccountBaseRewardReceiver extends string | IAccountMeta<string> = string,
  TAccountBaseFeeWallet extends string | IAccountMeta<string> = string,
  TAccountBaseFeeWalletAta extends string | IAccountMeta<string> = string,
  TAccountStakePoolProgram extends string | IAccountMeta<string> = string,
  TAccountStakePool extends string | IAccountMeta<string> = string,
  TAccountStakePoolWithdrawAuthority extends
    | string
    | IAccountMeta<string> = string,
  TAccountReserveStake extends string | IAccountMeta<string> = string,
  TAccountManagerFeeAccount extends string | IAccountMeta<string> = string,
  TAccountReferrerPoolTokensAccount extends
    | string
    | IAccountMeta<string> = string,
  TAccountPoolMint extends string | IAccountMeta<string> = string,
  TAccountTokenProgram extends
    | string
    | IAccountMeta<string> = 'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountEpochState extends string
        ? WritableAccount<TAccountEpochState>
        : TAccountEpochState,
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountBaseRewardRouter extends string
        ? WritableAccount<TAccountBaseRewardRouter>
        : TAccountBaseRewardRouter,
      TAccountBaseRewardReceiver extends string
        ? WritableAccount<TAccountBaseRewardReceiver>
        : TAccountBaseRewardReceiver,
      TAccountBaseFeeWallet extends string
        ? ReadonlyAccount<TAccountBaseFeeWallet>
        : TAccountBaseFeeWallet,
      TAccountBaseFeeWalletAta extends string
        ? WritableAccount<TAccountBaseFeeWalletAta>
        : TAccountBaseFeeWalletAta,
      TAccountStakePoolProgram extends string
        ? ReadonlyAccount<TAccountStakePoolProgram>
        : TAccountStakePoolProgram,
      TAccountStakePool extends string
        ? WritableAccount<TAccountStakePool>
        : TAccountStakePool,
      TAccountStakePoolWithdrawAuthority extends string
        ? ReadonlyAccount<TAccountStakePoolWithdrawAuthority>
        : TAccountStakePoolWithdrawAuthority,
      TAccountReserveStake extends string
        ? WritableAccount<TAccountReserveStake>
        : TAccountReserveStake,
      TAccountManagerFeeAccount extends string
        ? WritableAccount<TAccountManagerFeeAccount>
        : TAccountManagerFeeAccount,
      TAccountReferrerPoolTokensAccount extends string
        ? WritableAccount<TAccountReferrerPoolTokensAccount>
        : TAccountReferrerPoolTokensAccount,
      TAccountPoolMint extends string
        ? WritableAccount<TAccountPoolMint>
        : TAccountPoolMint,
      TAccountTokenProgram extends string
        ? ReadonlyAccount<TAccountTokenProgram>
        : TAccountTokenProgram,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type DistributeBaseRewardsInstructionData = {
  discriminator: number;
  baseFeeGroup: number;
  epoch: bigint;
};

export type DistributeBaseRewardsInstructionDataArgs = {
  baseFeeGroup: number;
  epoch: number | bigint;
};

export function getDistributeBaseRewardsInstructionDataEncoder(): Encoder<DistributeBaseRewardsInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['baseFeeGroup', getU8Encoder()],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({
      ...value,
      discriminator: DISTRIBUTE_BASE_REWARDS_DISCRIMINATOR,
    })
  );
}

export function getDistributeBaseRewardsInstructionDataDecoder(): Decoder<DistributeBaseRewardsInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['baseFeeGroup', getU8Decoder()],
    ['epoch', getU64Decoder()],
  ]);
}

export function getDistributeBaseRewardsInstructionDataCodec(): Codec<
  DistributeBaseRewardsInstructionDataArgs,
  DistributeBaseRewardsInstructionData
> {
  return combineCodec(
    getDistributeBaseRewardsInstructionDataEncoder(),
    getDistributeBaseRewardsInstructionDataDecoder()
  );
}

export type DistributeBaseRewardsInput<
  TAccountEpochState extends string = string,
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountBaseRewardRouter extends string = string,
  TAccountBaseRewardReceiver extends string = string,
  TAccountBaseFeeWallet extends string = string,
  TAccountBaseFeeWalletAta extends string = string,
  TAccountStakePoolProgram extends string = string,
  TAccountStakePool extends string = string,
  TAccountStakePoolWithdrawAuthority extends string = string,
  TAccountReserveStake extends string = string,
  TAccountManagerFeeAccount extends string = string,
  TAccountReferrerPoolTokensAccount extends string = string,
  TAccountPoolMint extends string = string,
  TAccountTokenProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  epochState: Address<TAccountEpochState>;
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  baseRewardRouter: Address<TAccountBaseRewardRouter>;
  baseRewardReceiver: Address<TAccountBaseRewardReceiver>;
  baseFeeWallet: Address<TAccountBaseFeeWallet>;
  baseFeeWalletAta: Address<TAccountBaseFeeWalletAta>;
  stakePoolProgram: Address<TAccountStakePoolProgram>;
  stakePool: Address<TAccountStakePool>;
  stakePoolWithdrawAuthority: Address<TAccountStakePoolWithdrawAuthority>;
  reserveStake: Address<TAccountReserveStake>;
  managerFeeAccount: Address<TAccountManagerFeeAccount>;
  referrerPoolTokensAccount: Address<TAccountReferrerPoolTokensAccount>;
  poolMint: Address<TAccountPoolMint>;
  tokenProgram?: Address<TAccountTokenProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  baseFeeGroup: DistributeBaseRewardsInstructionDataArgs['baseFeeGroup'];
  epoch: DistributeBaseRewardsInstructionDataArgs['epoch'];
};

export function getDistributeBaseRewardsInstruction<
  TAccountEpochState extends string,
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountBaseRewardRouter extends string,
  TAccountBaseRewardReceiver extends string,
  TAccountBaseFeeWallet extends string,
  TAccountBaseFeeWalletAta extends string,
  TAccountStakePoolProgram extends string,
  TAccountStakePool extends string,
  TAccountStakePoolWithdrawAuthority extends string,
  TAccountReserveStake extends string,
  TAccountManagerFeeAccount extends string,
  TAccountReferrerPoolTokensAccount extends string,
  TAccountPoolMint extends string,
  TAccountTokenProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: DistributeBaseRewardsInput<
    TAccountEpochState,
    TAccountConfig,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountBaseRewardReceiver,
    TAccountBaseFeeWallet,
    TAccountBaseFeeWalletAta,
    TAccountStakePoolProgram,
    TAccountStakePool,
    TAccountStakePoolWithdrawAuthority,
    TAccountReserveStake,
    TAccountManagerFeeAccount,
    TAccountReferrerPoolTokensAccount,
    TAccountPoolMint,
    TAccountTokenProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): DistributeBaseRewardsInstruction<
  TProgramAddress,
  TAccountEpochState,
  TAccountConfig,
  TAccountNcn,
  TAccountBaseRewardRouter,
  TAccountBaseRewardReceiver,
  TAccountBaseFeeWallet,
  TAccountBaseFeeWalletAta,
  TAccountStakePoolProgram,
  TAccountStakePool,
  TAccountStakePoolWithdrawAuthority,
  TAccountReserveStake,
  TAccountManagerFeeAccount,
  TAccountReferrerPoolTokensAccount,
  TAccountPoolMint,
  TAccountTokenProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    epochState: { value: input.epochState ?? null, isWritable: true },
    config: { value: input.config ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    baseRewardRouter: {
      value: input.baseRewardRouter ?? null,
      isWritable: true,
    },
    baseRewardReceiver: {
      value: input.baseRewardReceiver ?? null,
      isWritable: true,
    },
    baseFeeWallet: { value: input.baseFeeWallet ?? null, isWritable: false },
    baseFeeWalletAta: {
      value: input.baseFeeWalletAta ?? null,
      isWritable: true,
    },
    stakePoolProgram: {
      value: input.stakePoolProgram ?? null,
      isWritable: false,
    },
    stakePool: { value: input.stakePool ?? null, isWritable: true },
    stakePoolWithdrawAuthority: {
      value: input.stakePoolWithdrawAuthority ?? null,
      isWritable: false,
    },
    reserveStake: { value: input.reserveStake ?? null, isWritable: true },
    managerFeeAccount: {
      value: input.managerFeeAccount ?? null,
      isWritable: true,
    },
    referrerPoolTokensAccount: {
      value: input.referrerPoolTokensAccount ?? null,
      isWritable: true,
    },
    poolMint: { value: input.poolMint ?? null, isWritable: true },
    tokenProgram: { value: input.tokenProgram ?? null, isWritable: false },
    systemProgram: { value: input.systemProgram ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  // Original args.
  const args = { ...input };

  // Resolve default values.
  if (!accounts.tokenProgram.value) {
    accounts.tokenProgram.value =
      'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA' as Address<'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'>;
  }
  if (!accounts.systemProgram.value) {
    accounts.systemProgram.value =
      '11111111111111111111111111111111' as Address<'11111111111111111111111111111111'>;
  }

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.epochState),
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.baseRewardRouter),
      getAccountMeta(accounts.baseRewardReceiver),
      getAccountMeta(accounts.baseFeeWallet),
      getAccountMeta(accounts.baseFeeWalletAta),
      getAccountMeta(accounts.stakePoolProgram),
      getAccountMeta(accounts.stakePool),
      getAccountMeta(accounts.stakePoolWithdrawAuthority),
      getAccountMeta(accounts.reserveStake),
      getAccountMeta(accounts.managerFeeAccount),
      getAccountMeta(accounts.referrerPoolTokensAccount),
      getAccountMeta(accounts.poolMint),
      getAccountMeta(accounts.tokenProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getDistributeBaseRewardsInstructionDataEncoder().encode(
      args as DistributeBaseRewardsInstructionDataArgs
    ),
  } as DistributeBaseRewardsInstruction<
    TProgramAddress,
    TAccountEpochState,
    TAccountConfig,
    TAccountNcn,
    TAccountBaseRewardRouter,
    TAccountBaseRewardReceiver,
    TAccountBaseFeeWallet,
    TAccountBaseFeeWalletAta,
    TAccountStakePoolProgram,
    TAccountStakePool,
    TAccountStakePoolWithdrawAuthority,
    TAccountReserveStake,
    TAccountManagerFeeAccount,
    TAccountReferrerPoolTokensAccount,
    TAccountPoolMint,
    TAccountTokenProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedDistributeBaseRewardsInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    epochState: TAccountMetas[0];
    config: TAccountMetas[1];
    ncn: TAccountMetas[2];
    baseRewardRouter: TAccountMetas[3];
    baseRewardReceiver: TAccountMetas[4];
    baseFeeWallet: TAccountMetas[5];
    baseFeeWalletAta: TAccountMetas[6];
    stakePoolProgram: TAccountMetas[7];
    stakePool: TAccountMetas[8];
    stakePoolWithdrawAuthority: TAccountMetas[9];
    reserveStake: TAccountMetas[10];
    managerFeeAccount: TAccountMetas[11];
    referrerPoolTokensAccount: TAccountMetas[12];
    poolMint: TAccountMetas[13];
    tokenProgram: TAccountMetas[14];
    systemProgram: TAccountMetas[15];
  };
  data: DistributeBaseRewardsInstructionData;
};

export function parseDistributeBaseRewardsInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedDistributeBaseRewardsInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 16) {
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
      config: getNextAccount(),
      ncn: getNextAccount(),
      baseRewardRouter: getNextAccount(),
      baseRewardReceiver: getNextAccount(),
      baseFeeWallet: getNextAccount(),
      baseFeeWalletAta: getNextAccount(),
      stakePoolProgram: getNextAccount(),
      stakePool: getNextAccount(),
      stakePoolWithdrawAuthority: getNextAccount(),
      reserveStake: getNextAccount(),
      managerFeeAccount: getNextAccount(),
      referrerPoolTokensAccount: getNextAccount(),
      poolMint: getNextAccount(),
      tokenProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getDistributeBaseRewardsInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}

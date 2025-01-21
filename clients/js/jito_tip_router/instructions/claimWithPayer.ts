/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  fixDecoderSize,
  fixEncoderSize,
  getArrayDecoder,
  getArrayEncoder,
  getBytesDecoder,
  getBytesEncoder,
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
  type ReadonlyUint8Array,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const CLAIM_WITH_PAYER_DISCRIMINATOR = 26;

export function getClaimWithPayerDiscriminatorBytes() {
  return getU8Encoder().encode(CLAIM_WITH_PAYER_DISCRIMINATOR);
}

export type ClaimWithPayerInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountAccountPayer extends string | IAccountMeta<string> = string,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountTipDistributionConfig extends string | IAccountMeta<string> = string,
  TAccountTipDistributionAccount extends string | IAccountMeta<string> = string,
  TAccountClaimStatus extends string | IAccountMeta<string> = string,
  TAccountClaimant extends string | IAccountMeta<string> = string,
  TAccountTipDistributionProgram extends string | IAccountMeta<string> = string,
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountAccountPayer extends string
        ? WritableAccount<TAccountAccountPayer>
        : TAccountAccountPayer,
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountTipDistributionConfig extends string
        ? ReadonlyAccount<TAccountTipDistributionConfig>
        : TAccountTipDistributionConfig,
      TAccountTipDistributionAccount extends string
        ? WritableAccount<TAccountTipDistributionAccount>
        : TAccountTipDistributionAccount,
      TAccountClaimStatus extends string
        ? WritableAccount<TAccountClaimStatus>
        : TAccountClaimStatus,
      TAccountClaimant extends string
        ? WritableAccount<TAccountClaimant>
        : TAccountClaimant,
      TAccountTipDistributionProgram extends string
        ? ReadonlyAccount<TAccountTipDistributionProgram>
        : TAccountTipDistributionProgram,
      TAccountSystemProgram extends string
        ? ReadonlyAccount<TAccountSystemProgram>
        : TAccountSystemProgram,
      ...TRemainingAccounts,
    ]
  >;

export type ClaimWithPayerInstructionData = {
  discriminator: number;
  proof: Array<ReadonlyUint8Array>;
  amount: bigint;
  bump: number;
};

export type ClaimWithPayerInstructionDataArgs = {
  proof: Array<ReadonlyUint8Array>;
  amount: number | bigint;
  bump: number;
};

export function getClaimWithPayerInstructionDataEncoder(): Encoder<ClaimWithPayerInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['proof', getArrayEncoder(fixEncoderSize(getBytesEncoder(), 32))],
      ['amount', getU64Encoder()],
      ['bump', getU8Encoder()],
    ]),
    (value) => ({ ...value, discriminator: CLAIM_WITH_PAYER_DISCRIMINATOR })
  );
}

export function getClaimWithPayerInstructionDataDecoder(): Decoder<ClaimWithPayerInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['proof', getArrayDecoder(fixDecoderSize(getBytesDecoder(), 32))],
    ['amount', getU64Decoder()],
    ['bump', getU8Decoder()],
  ]);
}

export function getClaimWithPayerInstructionDataCodec(): Codec<
  ClaimWithPayerInstructionDataArgs,
  ClaimWithPayerInstructionData
> {
  return combineCodec(
    getClaimWithPayerInstructionDataEncoder(),
    getClaimWithPayerInstructionDataDecoder()
  );
}

export type ClaimWithPayerInput<
  TAccountAccountPayer extends string = string,
  TAccountConfig extends string = string,
  TAccountNcn extends string = string,
  TAccountTipDistributionConfig extends string = string,
  TAccountTipDistributionAccount extends string = string,
  TAccountClaimStatus extends string = string,
  TAccountClaimant extends string = string,
  TAccountTipDistributionProgram extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  accountPayer: Address<TAccountAccountPayer>;
  config: Address<TAccountConfig>;
  ncn: Address<TAccountNcn>;
  tipDistributionConfig: Address<TAccountTipDistributionConfig>;
  tipDistributionAccount: Address<TAccountTipDistributionAccount>;
  claimStatus: Address<TAccountClaimStatus>;
  claimant: Address<TAccountClaimant>;
  tipDistributionProgram: Address<TAccountTipDistributionProgram>;
  systemProgram?: Address<TAccountSystemProgram>;
  proof: ClaimWithPayerInstructionDataArgs['proof'];
  amount: ClaimWithPayerInstructionDataArgs['amount'];
  bump: ClaimWithPayerInstructionDataArgs['bump'];
};

export function getClaimWithPayerInstruction<
  TAccountAccountPayer extends string,
  TAccountConfig extends string,
  TAccountNcn extends string,
  TAccountTipDistributionConfig extends string,
  TAccountTipDistributionAccount extends string,
  TAccountClaimStatus extends string,
  TAccountClaimant extends string,
  TAccountTipDistributionProgram extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: ClaimWithPayerInput<
    TAccountAccountPayer,
    TAccountConfig,
    TAccountNcn,
    TAccountTipDistributionConfig,
    TAccountTipDistributionAccount,
    TAccountClaimStatus,
    TAccountClaimant,
    TAccountTipDistributionProgram,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): ClaimWithPayerInstruction<
  TProgramAddress,
  TAccountAccountPayer,
  TAccountConfig,
  TAccountNcn,
  TAccountTipDistributionConfig,
  TAccountTipDistributionAccount,
  TAccountClaimStatus,
  TAccountClaimant,
  TAccountTipDistributionProgram,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    accountPayer: { value: input.accountPayer ?? null, isWritable: true },
    config: { value: input.config ?? null, isWritable: false },
    ncn: { value: input.ncn ?? null, isWritable: false },
    tipDistributionConfig: {
      value: input.tipDistributionConfig ?? null,
      isWritable: false,
    },
    tipDistributionAccount: {
      value: input.tipDistributionAccount ?? null,
      isWritable: true,
    },
    claimStatus: { value: input.claimStatus ?? null, isWritable: true },
    claimant: { value: input.claimant ?? null, isWritable: true },
    tipDistributionProgram: {
      value: input.tipDistributionProgram ?? null,
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
      getAccountMeta(accounts.accountPayer),
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.tipDistributionConfig),
      getAccountMeta(accounts.tipDistributionAccount),
      getAccountMeta(accounts.claimStatus),
      getAccountMeta(accounts.claimant),
      getAccountMeta(accounts.tipDistributionProgram),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getClaimWithPayerInstructionDataEncoder().encode(
      args as ClaimWithPayerInstructionDataArgs
    ),
  } as ClaimWithPayerInstruction<
    TProgramAddress,
    TAccountAccountPayer,
    TAccountConfig,
    TAccountNcn,
    TAccountTipDistributionConfig,
    TAccountTipDistributionAccount,
    TAccountClaimStatus,
    TAccountClaimant,
    TAccountTipDistributionProgram,
    TAccountSystemProgram
  >;

  return instruction;
}

export type ParsedClaimWithPayerInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    accountPayer: TAccountMetas[0];
    config: TAccountMetas[1];
    ncn: TAccountMetas[2];
    tipDistributionConfig: TAccountMetas[3];
    tipDistributionAccount: TAccountMetas[4];
    claimStatus: TAccountMetas[5];
    claimant: TAccountMetas[6];
    tipDistributionProgram: TAccountMetas[7];
    systemProgram: TAccountMetas[8];
  };
  data: ClaimWithPayerInstructionData;
};

export function parseClaimWithPayerInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedClaimWithPayerInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 9) {
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
      accountPayer: getNextAccount(),
      config: getNextAccount(),
      ncn: getNextAccount(),
      tipDistributionConfig: getNextAccount(),
      tipDistributionAccount: getNextAccount(),
      claimStatus: getNextAccount(),
      claimant: getNextAccount(),
      tipDistributionProgram: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getClaimWithPayerInstructionDataDecoder().decode(instruction.data),
  };
}

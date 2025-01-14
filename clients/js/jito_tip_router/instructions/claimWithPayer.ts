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
  TAccountClaimStatusPayer extends string | IAccountMeta<string> = string,
  TAccountTipDistributionProgram extends string | IAccountMeta<string> = string,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountTipDistributionAccount extends string | IAccountMeta<string> = string,
  TAccountClaimStatus extends string | IAccountMeta<string> = string,
  TAccountClaimant extends string | IAccountMeta<string> = string,
  TAccountSystemProgram extends
    | string
    | IAccountMeta<string> = '11111111111111111111111111111111',
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountClaimStatusPayer extends string
        ? WritableAccount<TAccountClaimStatusPayer>
        : TAccountClaimStatusPayer,
      TAccountTipDistributionProgram extends string
        ? ReadonlyAccount<TAccountTipDistributionProgram>
        : TAccountTipDistributionProgram,
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountTipDistributionAccount extends string
        ? WritableAccount<TAccountTipDistributionAccount>
        : TAccountTipDistributionAccount,
      TAccountClaimStatus extends string
        ? WritableAccount<TAccountClaimStatus>
        : TAccountClaimStatus,
      TAccountClaimant extends string
        ? WritableAccount<TAccountClaimant>
        : TAccountClaimant,
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
  TAccountClaimStatusPayer extends string = string,
  TAccountTipDistributionProgram extends string = string,
  TAccountConfig extends string = string,
  TAccountTipDistributionAccount extends string = string,
  TAccountClaimStatus extends string = string,
  TAccountClaimant extends string = string,
  TAccountSystemProgram extends string = string,
> = {
  claimStatusPayer: Address<TAccountClaimStatusPayer>;
  tipDistributionProgram: Address<TAccountTipDistributionProgram>;
  config: Address<TAccountConfig>;
  tipDistributionAccount: Address<TAccountTipDistributionAccount>;
  claimStatus: Address<TAccountClaimStatus>;
  claimant: Address<TAccountClaimant>;
  systemProgram?: Address<TAccountSystemProgram>;
  proof: ClaimWithPayerInstructionDataArgs['proof'];
  amount: ClaimWithPayerInstructionDataArgs['amount'];
  bump: ClaimWithPayerInstructionDataArgs['bump'];
};

export function getClaimWithPayerInstruction<
  TAccountClaimStatusPayer extends string,
  TAccountTipDistributionProgram extends string,
  TAccountConfig extends string,
  TAccountTipDistributionAccount extends string,
  TAccountClaimStatus extends string,
  TAccountClaimant extends string,
  TAccountSystemProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: ClaimWithPayerInput<
    TAccountClaimStatusPayer,
    TAccountTipDistributionProgram,
    TAccountConfig,
    TAccountTipDistributionAccount,
    TAccountClaimStatus,
    TAccountClaimant,
    TAccountSystemProgram
  >,
  config?: { programAddress?: TProgramAddress }
): ClaimWithPayerInstruction<
  TProgramAddress,
  TAccountClaimStatusPayer,
  TAccountTipDistributionProgram,
  TAccountConfig,
  TAccountTipDistributionAccount,
  TAccountClaimStatus,
  TAccountClaimant,
  TAccountSystemProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    claimStatusPayer: {
      value: input.claimStatusPayer ?? null,
      isWritable: true,
    },
    tipDistributionProgram: {
      value: input.tipDistributionProgram ?? null,
      isWritable: false,
    },
    config: { value: input.config ?? null, isWritable: false },
    tipDistributionAccount: {
      value: input.tipDistributionAccount ?? null,
      isWritable: true,
    },
    claimStatus: { value: input.claimStatus ?? null, isWritable: true },
    claimant: { value: input.claimant ?? null, isWritable: true },
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
      getAccountMeta(accounts.claimStatusPayer),
      getAccountMeta(accounts.tipDistributionProgram),
      getAccountMeta(accounts.config),
      getAccountMeta(accounts.tipDistributionAccount),
      getAccountMeta(accounts.claimStatus),
      getAccountMeta(accounts.claimant),
      getAccountMeta(accounts.systemProgram),
    ],
    programAddress,
    data: getClaimWithPayerInstructionDataEncoder().encode(
      args as ClaimWithPayerInstructionDataArgs
    ),
  } as ClaimWithPayerInstruction<
    TProgramAddress,
    TAccountClaimStatusPayer,
    TAccountTipDistributionProgram,
    TAccountConfig,
    TAccountTipDistributionAccount,
    TAccountClaimStatus,
    TAccountClaimant,
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
    claimStatusPayer: TAccountMetas[0];
    tipDistributionProgram: TAccountMetas[1];
    config: TAccountMetas[2];
    tipDistributionAccount: TAccountMetas[3];
    claimStatus: TAccountMetas[4];
    claimant: TAccountMetas[5];
    systemProgram: TAccountMetas[6];
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
      claimStatusPayer: getNextAccount(),
      tipDistributionProgram: getNextAccount(),
      config: getNextAccount(),
      tipDistributionAccount: getNextAccount(),
      claimStatus: getNextAccount(),
      claimant: getNextAccount(),
      systemProgram: getNextAccount(),
    },
    data: getClaimWithPayerInstructionDataDecoder().decode(instruction.data),
  };
}

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
  type IAccountSignerMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type ReadonlyAccount,
  type ReadonlySignerAccount,
  type ReadonlyUint8Array,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const CAST_VOTE_DISCRIMINATOR = 13;

export function getCastVoteDiscriminatorBytes() {
  return getU8Encoder().encode(CAST_VOTE_DISCRIMINATOR);
}

export type CastVoteInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountConfig extends string | IAccountMeta<string> = string,
  TAccountBallotBox extends string | IAccountMeta<string> = string,
  TAccountNcn extends string | IAccountMeta<string> = string,
  TAccountEpochSnapshot extends string | IAccountMeta<string> = string,
  TAccountOperatorSnapshot extends string | IAccountMeta<string> = string,
  TAccountOperator extends string | IAccountMeta<string> = string,
  TAccountOperatorAdmin extends string | IAccountMeta<string> = string,
  TAccountRestakingProgram extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountConfig extends string
        ? ReadonlyAccount<TAccountConfig>
        : TAccountConfig,
      TAccountBallotBox extends string
        ? WritableAccount<TAccountBallotBox>
        : TAccountBallotBox,
      TAccountNcn extends string ? ReadonlyAccount<TAccountNcn> : TAccountNcn,
      TAccountEpochSnapshot extends string
        ? ReadonlyAccount<TAccountEpochSnapshot>
        : TAccountEpochSnapshot,
      TAccountOperatorSnapshot extends string
        ? ReadonlyAccount<TAccountOperatorSnapshot>
        : TAccountOperatorSnapshot,
      TAccountOperator extends string
        ? ReadonlyAccount<TAccountOperator>
        : TAccountOperator,
      TAccountOperatorAdmin extends string
        ? ReadonlySignerAccount<TAccountOperatorAdmin> &
            IAccountSignerMeta<TAccountOperatorAdmin>
        : TAccountOperatorAdmin,
      TAccountRestakingProgram extends string
        ? ReadonlyAccount<TAccountRestakingProgram>
        : TAccountRestakingProgram,
      ...TRemainingAccounts,
    ]
  >;

export type CastVoteInstructionData = {
  discriminator: number;
  metaMerkleRoot: ReadonlyUint8Array;
  epoch: bigint;
};

export type CastVoteInstructionDataArgs = {
  metaMerkleRoot: ReadonlyUint8Array;
  epoch: number | bigint;
};

export function getCastVoteInstructionDataEncoder(): Encoder<CastVoteInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU8Encoder()],
      ['metaMerkleRoot', fixEncoderSize(getBytesEncoder(), 32)],
      ['epoch', getU64Encoder()],
    ]),
    (value) => ({ ...value, discriminator: CAST_VOTE_DISCRIMINATOR })
  );
}

export function getCastVoteInstructionDataDecoder(): Decoder<CastVoteInstructionData> {
  return getStructDecoder([
    ['discriminator', getU8Decoder()],
    ['metaMerkleRoot', fixDecoderSize(getBytesDecoder(), 32)],
    ['epoch', getU64Decoder()],
  ]);
}

export function getCastVoteInstructionDataCodec(): Codec<
  CastVoteInstructionDataArgs,
  CastVoteInstructionData
> {
  return combineCodec(
    getCastVoteInstructionDataEncoder(),
    getCastVoteInstructionDataDecoder()
  );
}

export type CastVoteInput<
  TAccountConfig extends string = string,
  TAccountBallotBox extends string = string,
  TAccountNcn extends string = string,
  TAccountEpochSnapshot extends string = string,
  TAccountOperatorSnapshot extends string = string,
  TAccountOperator extends string = string,
  TAccountOperatorAdmin extends string = string,
  TAccountRestakingProgram extends string = string,
> = {
  config: Address<TAccountConfig>;
  ballotBox: Address<TAccountBallotBox>;
  ncn: Address<TAccountNcn>;
  epochSnapshot: Address<TAccountEpochSnapshot>;
  operatorSnapshot: Address<TAccountOperatorSnapshot>;
  operator: Address<TAccountOperator>;
  operatorAdmin: TransactionSigner<TAccountOperatorAdmin>;
  restakingProgram: Address<TAccountRestakingProgram>;
  metaMerkleRoot: CastVoteInstructionDataArgs['metaMerkleRoot'];
  epoch: CastVoteInstructionDataArgs['epoch'];
};

export function getCastVoteInstruction<
  TAccountConfig extends string,
  TAccountBallotBox extends string,
  TAccountNcn extends string,
  TAccountEpochSnapshot extends string,
  TAccountOperatorSnapshot extends string,
  TAccountOperator extends string,
  TAccountOperatorAdmin extends string,
  TAccountRestakingProgram extends string,
  TProgramAddress extends Address = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
>(
  input: CastVoteInput<
    TAccountConfig,
    TAccountBallotBox,
    TAccountNcn,
    TAccountEpochSnapshot,
    TAccountOperatorSnapshot,
    TAccountOperator,
    TAccountOperatorAdmin,
    TAccountRestakingProgram
  >,
  config?: { programAddress?: TProgramAddress }
): CastVoteInstruction<
  TProgramAddress,
  TAccountConfig,
  TAccountBallotBox,
  TAccountNcn,
  TAccountEpochSnapshot,
  TAccountOperatorSnapshot,
  TAccountOperator,
  TAccountOperatorAdmin,
  TAccountRestakingProgram
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? JITO_TIP_ROUTER_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    config: { value: input.config ?? null, isWritable: false },
    ballotBox: { value: input.ballotBox ?? null, isWritable: true },
    ncn: { value: input.ncn ?? null, isWritable: false },
    epochSnapshot: { value: input.epochSnapshot ?? null, isWritable: false },
    operatorSnapshot: {
      value: input.operatorSnapshot ?? null,
      isWritable: false,
    },
    operator: { value: input.operator ?? null, isWritable: false },
    operatorAdmin: { value: input.operatorAdmin ?? null, isWritable: false },
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
      getAccountMeta(accounts.ballotBox),
      getAccountMeta(accounts.ncn),
      getAccountMeta(accounts.epochSnapshot),
      getAccountMeta(accounts.operatorSnapshot),
      getAccountMeta(accounts.operator),
      getAccountMeta(accounts.operatorAdmin),
      getAccountMeta(accounts.restakingProgram),
    ],
    programAddress,
    data: getCastVoteInstructionDataEncoder().encode(
      args as CastVoteInstructionDataArgs
    ),
  } as CastVoteInstruction<
    TProgramAddress,
    TAccountConfig,
    TAccountBallotBox,
    TAccountNcn,
    TAccountEpochSnapshot,
    TAccountOperatorSnapshot,
    TAccountOperator,
    TAccountOperatorAdmin,
    TAccountRestakingProgram
  >;

  return instruction;
}

export type ParsedCastVoteInstruction<
  TProgram extends string = typeof JITO_TIP_ROUTER_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    config: TAccountMetas[0];
    ballotBox: TAccountMetas[1];
    ncn: TAccountMetas[2];
    epochSnapshot: TAccountMetas[3];
    operatorSnapshot: TAccountMetas[4];
    operator: TAccountMetas[5];
    operatorAdmin: TAccountMetas[6];
    restakingProgram: TAccountMetas[7];
  };
  data: CastVoteInstructionData;
};

export function parseCastVoteInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedCastVoteInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 8) {
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
      ballotBox: getNextAccount(),
      ncn: getNextAccount(),
      epochSnapshot: getNextAccount(),
      operatorSnapshot: getNextAccount(),
      operator: getNextAccount(),
      operatorAdmin: getNextAccount(),
      restakingProgram: getNextAccount(),
    },
    data: getCastVoteInstructionDataDecoder().decode(instruction.data),
  };
}

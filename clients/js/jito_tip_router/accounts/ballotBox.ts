/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  assertAccountExists,
  assertAccountsExist,
  combineCodec,
  decodeAccount,
  fetchEncodedAccount,
  fetchEncodedAccounts,
  getAddressDecoder,
  getAddressEncoder,
  getArrayDecoder,
  getArrayEncoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Account,
  type Address,
  type Codec,
  type Decoder,
  type EncodedAccount,
  type Encoder,
  type FetchAccountConfig,
  type FetchAccountsConfig,
  type MaybeAccount,
  type MaybeEncodedAccount,
} from '@solana/web3.js';
import {
  getBallotTallyDecoder,
  getBallotTallyEncoder,
  getOperatorVoteDecoder,
  getOperatorVoteEncoder,
  type BallotTally,
  type BallotTallyArgs,
  type OperatorVote,
  type OperatorVoteArgs,
} from '../types';

export type BallotBox = {
  discriminator: bigint;
  ncn: Address;
  epoch: bigint;
  bump: number;
  slotCreated: bigint;
  slotConsensusReached: bigint;
  reserved: Array<number>;
  operatorsVoted: bigint;
  uniqueBallots: bigint;
  winningBallot: BallotTally;
  operatorVotes: Array<OperatorVote>;
  ballotTallies: Array<BallotTally>;
};

export type BallotBoxArgs = {
  discriminator: number | bigint;
  ncn: Address;
  epoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  slotConsensusReached: number | bigint;
  reserved: Array<number>;
  operatorsVoted: number | bigint;
  uniqueBallots: number | bigint;
  winningBallot: BallotTallyArgs;
  operatorVotes: Array<OperatorVoteArgs>;
  ballotTallies: Array<BallotTallyArgs>;
};

export function getBallotBoxEncoder(): Encoder<BallotBoxArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['epoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['slotConsensusReached', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
    ['operatorsVoted', getU64Encoder()],
    ['uniqueBallots', getU64Encoder()],
    ['winningBallot', getBallotTallyEncoder()],
    ['operatorVotes', getArrayEncoder(getOperatorVoteEncoder(), { size: 32 })],
    ['ballotTallies', getArrayEncoder(getBallotTallyEncoder(), { size: 32 })],
  ]);
}

export function getBallotBoxDecoder(): Decoder<BallotBox> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['epoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['slotConsensusReached', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
    ['operatorsVoted', getU64Decoder()],
    ['uniqueBallots', getU64Decoder()],
    ['winningBallot', getBallotTallyDecoder()],
    ['operatorVotes', getArrayDecoder(getOperatorVoteDecoder(), { size: 32 })],
    ['ballotTallies', getArrayDecoder(getBallotTallyDecoder(), { size: 32 })],
  ]);
}

export function getBallotBoxCodec(): Codec<BallotBoxArgs, BallotBox> {
  return combineCodec(getBallotBoxEncoder(), getBallotBoxDecoder());
}

export function decodeBallotBox<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<BallotBox, TAddress>;
export function decodeBallotBox<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<BallotBox, TAddress>;
export function decodeBallotBox<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
): Account<BallotBox, TAddress> | MaybeAccount<BallotBox, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getBallotBoxDecoder()
  );
}

export async function fetchBallotBox<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<BallotBox, TAddress>> {
  const maybeAccount = await fetchMaybeBallotBox(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeBallotBox<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<BallotBox, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeBallotBox(maybeAccount);
}

export async function fetchAllBallotBox(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<BallotBox>[]> {
  const maybeAccounts = await fetchAllMaybeBallotBox(rpc, addresses, config);
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeBallotBox(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<BallotBox>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) => decodeBallotBox(maybeAccount));
}

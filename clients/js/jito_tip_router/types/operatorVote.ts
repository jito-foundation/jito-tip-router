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
  getAddressDecoder,
  getAddressEncoder,
  getBytesDecoder,
  getBytesEncoder,
  getStructDecoder,
  getStructEncoder,
  getU16Decoder,
  getU16Encoder,
  getU64Decoder,
  getU64Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type ReadonlyUint8Array,
} from '@solana/web3.js';
import {
  getStakeWeightsDecoder,
  getStakeWeightsEncoder,
  type StakeWeights,
  type StakeWeightsArgs,
} from '.';

export type OperatorVote = {
  operator: Address;
  slotVoted: bigint;
  stakeWeights: StakeWeights;
  ballotIndex: number;
  reserved: ReadonlyUint8Array;
};

export type OperatorVoteArgs = {
  operator: Address;
  slotVoted: number | bigint;
  stakeWeights: StakeWeightsArgs;
  ballotIndex: number;
  reserved: ReadonlyUint8Array;
};

export function getOperatorVoteEncoder(): Encoder<OperatorVoteArgs> {
  return getStructEncoder([
    ['operator', getAddressEncoder()],
    ['slotVoted', getU64Encoder()],
    ['stakeWeights', getStakeWeightsEncoder()],
    ['ballotIndex', getU16Encoder()],
    ['reserved', fixEncoderSize(getBytesEncoder(), 64)],
  ]);
}

export function getOperatorVoteDecoder(): Decoder<OperatorVote> {
  return getStructDecoder([
    ['operator', getAddressDecoder()],
    ['slotVoted', getU64Decoder()],
    ['stakeWeights', getStakeWeightsDecoder()],
    ['ballotIndex', getU16Decoder()],
    ['reserved', fixDecoderSize(getBytesDecoder(), 64)],
  ]);
}

export function getOperatorVoteCodec(): Codec<OperatorVoteArgs, OperatorVote> {
  return combineCodec(getOperatorVoteEncoder(), getOperatorVoteDecoder());
}

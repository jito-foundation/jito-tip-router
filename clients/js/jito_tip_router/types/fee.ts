/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getAddressDecoder,
  getAddressEncoder,
  getStructDecoder,
  getStructEncoder,
  getU64Decoder,
  getU64Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';

export type Fee = {
  wallet: Address;
  daoShareBps: bigint;
  ncnShareBps: bigint;
  blockEngineFeeBps: bigint;
  activationEpoch: bigint;
};

export type FeeArgs = {
  wallet: Address;
  daoShareBps: number | bigint;
  ncnShareBps: number | bigint;
  blockEngineFeeBps: number | bigint;
  activationEpoch: number | bigint;
};

export function getFeeEncoder(): Encoder<FeeArgs> {
  return getStructEncoder([
    ['wallet', getAddressEncoder()],
    ['daoShareBps', getU64Encoder()],
    ['ncnShareBps', getU64Encoder()],
    ['blockEngineFeeBps', getU64Encoder()],
    ['activationEpoch', getU64Encoder()],
  ]);
}

export function getFeeDecoder(): Decoder<Fee> {
  return getStructDecoder([
    ['wallet', getAddressDecoder()],
    ['daoShareBps', getU64Decoder()],
    ['ncnShareBps', getU64Decoder()],
    ['blockEngineFeeBps', getU64Decoder()],
    ['activationEpoch', getU64Decoder()],
  ]);
}

export function getFeeCodec(): Codec<FeeArgs, Fee> {
  return combineCodec(getFeeEncoder(), getFeeDecoder());
}

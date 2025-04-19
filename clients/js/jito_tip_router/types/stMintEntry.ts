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
  getArrayDecoder,
  getArrayEncoder,
  getStructDecoder,
  getStructEncoder,
  getU128Decoder,
  getU128Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';

export type StMintEntry = {
  stMint: Address;
  rewardMultiplierBps: bigint;
  reservedRewardMultiplierBps: bigint;
  switchboardFeed: Address;
  noFeedWeight: bigint;
  reserved: Array<number>;
};

export type StMintEntryArgs = {
  stMint: Address;
  rewardMultiplierBps: number | bigint;
  reservedRewardMultiplierBps: number | bigint;
  switchboardFeed: Address;
  noFeedWeight: number | bigint;
  reserved: Array<number>;
};

export function getStMintEntryEncoder(): Encoder<StMintEntryArgs> {
  return getStructEncoder([
    ['stMint', getAddressEncoder()],
    ['rewardMultiplierBps', getU64Encoder()],
    ['reservedRewardMultiplierBps', getU64Encoder()],
    ['switchboardFeed', getAddressEncoder()],
    ['noFeedWeight', getU128Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
  ]);
}

export function getStMintEntryDecoder(): Decoder<StMintEntry> {
  return getStructDecoder([
    ['stMint', getAddressDecoder()],
    ['rewardMultiplierBps', getU64Decoder()],
    ['reservedRewardMultiplierBps', getU64Decoder()],
    ['switchboardFeed', getAddressDecoder()],
    ['noFeedWeight', getU128Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
  ]);
}

export function getStMintEntryCodec(): Codec<StMintEntryArgs, StMintEntry> {
  return combineCodec(getStMintEntryEncoder(), getStMintEntryDecoder());
}

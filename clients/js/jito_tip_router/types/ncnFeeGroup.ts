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
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';

export type NcnFeeGroup = { group: number };

export type NcnFeeGroupArgs = NcnFeeGroup;

export function getNcnFeeGroupEncoder(): Encoder<NcnFeeGroupArgs> {
  return getStructEncoder([['group', getU8Encoder()]]);
}

export function getNcnFeeGroupDecoder(): Decoder<NcnFeeGroup> {
  return getStructDecoder([['group', getU8Decoder()]]);
}

export function getNcnFeeGroupCodec(): Codec<NcnFeeGroupArgs, NcnFeeGroup> {
  return combineCodec(getNcnFeeGroupEncoder(), getNcnFeeGroupDecoder());
}

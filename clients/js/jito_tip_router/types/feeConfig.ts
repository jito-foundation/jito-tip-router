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
  getU16Decoder,
  getU16Encoder,
  getU8Decoder,
  getU8Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';
import { getFeesDecoder, getFeesEncoder, type Fees, type FeesArgs } from '.';

export type FeeConfig = {
  blockEngineFeeBps: number;
  baseFeeWallets: Array<Address>;
  reserved: Array<number>;
  fee1: Fees;
  fee2: Fees;
};

export type FeeConfigArgs = {
  blockEngineFeeBps: number;
  baseFeeWallets: Array<Address>;
  reserved: Array<number>;
  fee1: FeesArgs;
  fee2: FeesArgs;
};

export function getFeeConfigEncoder(): Encoder<FeeConfigArgs> {
  return getStructEncoder([
    ['blockEngineFeeBps', getU16Encoder()],
    ['baseFeeWallets', getArrayEncoder(getAddressEncoder(), { size: 8 })],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
    ['fee1', getFeesEncoder()],
    ['fee2', getFeesEncoder()],
  ]);
}

export function getFeeConfigDecoder(): Decoder<FeeConfig> {
  return getStructDecoder([
    ['blockEngineFeeBps', getU16Decoder()],
    ['baseFeeWallets', getArrayDecoder(getAddressDecoder(), { size: 8 })],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
    ['fee1', getFeesDecoder()],
    ['fee2', getFeesDecoder()],
  ]);
}

export function getFeeConfigCodec(): Codec<FeeConfigArgs, FeeConfig> {
  return combineCodec(getFeeConfigEncoder(), getFeeConfigDecoder());
}
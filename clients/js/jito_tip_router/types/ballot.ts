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
  getBoolDecoder,
  getBoolEncoder,
  getBytesDecoder,
  getBytesEncoder,
  getStructDecoder,
  getStructEncoder,
  getU8Decoder,
  getU8Encoder,
  type Codec,
  type Decoder,
  type Encoder,
  type ReadonlyUint8Array,
} from '@solana/web3.js';

export type Ballot = {
  metaMerkleRoot: ReadonlyUint8Array;
  isInitialized: number;
  reserved: Array<number>;
};

export type BallotArgs = Ballot;

export function getBallotEncoder(): Encoder<BallotArgs> {
  return getStructEncoder([
    ['metaMerkleRoot', fixEncoderSize(getBytesEncoder(), 32)],
    ['isInitialized', getBoolEncoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 63 })],
  ]);
}

export function getBallotDecoder(): Decoder<Ballot> {
  return getStructDecoder([
    ['metaMerkleRoot', fixDecoderSize(getBytesDecoder(), 32)],
    ['isInitialized', getBoolDecoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 63 })],
  ]);
}

export function getBallotCodec(): Codec<BallotArgs, Ballot> {
  return combineCodec(getBallotEncoder(), getBallotDecoder());
}

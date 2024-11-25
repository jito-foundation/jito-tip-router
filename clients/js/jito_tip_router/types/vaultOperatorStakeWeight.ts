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
  getU64Decoder,
  getU64Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type ReadonlyUint8Array,
} from '@solana/web3.js';
import {
  getStakeWeightDecoder,
  getStakeWeightEncoder,
  type StakeWeight,
  type StakeWeightArgs,
} from '.';

export type VaultOperatorStakeWeight = {
  vault: Address;
  vaultIndex: bigint;
  stakeWeight: StakeWeight;
  reserved: ReadonlyUint8Array;
};

export type VaultOperatorStakeWeightArgs = {
  vault: Address;
  vaultIndex: number | bigint;
  stakeWeight: StakeWeightArgs;
  reserved: ReadonlyUint8Array;
};

export function getVaultOperatorStakeWeightEncoder(): Encoder<VaultOperatorStakeWeightArgs> {
  return getStructEncoder([
    ['vault', getAddressEncoder()],
    ['vaultIndex', getU64Encoder()],
    ['stakeWeight', getStakeWeightEncoder()],
    ['reserved', fixEncoderSize(getBytesEncoder(), 32)],
  ]);
}

export function getVaultOperatorStakeWeightDecoder(): Decoder<VaultOperatorStakeWeight> {
  return getStructDecoder([
    ['vault', getAddressDecoder()],
    ['vaultIndex', getU64Decoder()],
    ['stakeWeight', getStakeWeightDecoder()],
    ['reserved', fixDecoderSize(getBytesDecoder(), 32)],
  ]);
}

export function getVaultOperatorStakeWeightCodec(): Codec<
  VaultOperatorStakeWeightArgs,
  VaultOperatorStakeWeight
> {
  return combineCodec(
    getVaultOperatorStakeWeightEncoder(),
    getVaultOperatorStakeWeightDecoder()
  );
}

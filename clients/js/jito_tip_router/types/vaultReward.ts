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
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';

export type VaultReward = {
  vault: Address;
  reward: bigint;
  reserved: Array<number>;
};

export type VaultRewardArgs = {
  vault: Address;
  reward: number | bigint;
  reserved: Array<number>;
};

export function getVaultRewardEncoder(): Encoder<VaultRewardArgs> {
  return getStructEncoder([
    ['vault', getAddressEncoder()],
    ['reward', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
  ]);
}

export function getVaultRewardDecoder(): Decoder<VaultReward> {
  return getStructDecoder([
    ['vault', getAddressDecoder()],
    ['reward', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
  ]);
}

export function getVaultRewardCodec(): Codec<VaultRewardArgs, VaultReward> {
  return combineCodec(getVaultRewardEncoder(), getVaultRewardDecoder());
}

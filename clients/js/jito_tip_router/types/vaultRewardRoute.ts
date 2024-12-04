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

export type VaultRewardRoute = { vault: Address; rewards: bigint };

export type VaultRewardRouteArgs = { vault: Address; rewards: number | bigint };

export function getVaultRewardRouteEncoder(): Encoder<VaultRewardRouteArgs> {
  return getStructEncoder([
    ['vault', getAddressEncoder()],
    ['rewards', getU64Encoder()],
  ]);
}

export function getVaultRewardRouteDecoder(): Decoder<VaultRewardRoute> {
  return getStructDecoder([
    ['vault', getAddressDecoder()],
    ['rewards', getU64Decoder()],
  ]);
}

export function getVaultRewardRouteCodec(): Codec<
  VaultRewardRouteArgs,
  VaultRewardRoute
> {
  return combineCodec(
    getVaultRewardRouteEncoder(),
    getVaultRewardRouteDecoder()
  );
}
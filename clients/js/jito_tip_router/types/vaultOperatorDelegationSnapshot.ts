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

export type VaultOperatorDelegationSnapshot = {
  vault: Address;
  stMint: Address;
  totalSecurity: bigint;
  totalVotes: bigint;
  slotSet: bigint;
  reserved: Array<number>;
};

export type VaultOperatorDelegationSnapshotArgs = {
  vault: Address;
  stMint: Address;
  totalSecurity: number | bigint;
  totalVotes: number | bigint;
  slotSet: number | bigint;
  reserved: Array<number>;
};

export function getVaultOperatorDelegationSnapshotEncoder(): Encoder<VaultOperatorDelegationSnapshotArgs> {
  return getStructEncoder([
    ['vault', getAddressEncoder()],
    ['stMint', getAddressEncoder()],
    ['totalSecurity', getU64Encoder()],
    ['totalVotes', getU128Encoder()],
    ['slotSet', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
  ]);
}

export function getVaultOperatorDelegationSnapshotDecoder(): Decoder<VaultOperatorDelegationSnapshot> {
  return getStructDecoder([
    ['vault', getAddressDecoder()],
    ['stMint', getAddressDecoder()],
    ['totalSecurity', getU64Decoder()],
    ['totalVotes', getU128Decoder()],
    ['slotSet', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
  ]);
}

export function getVaultOperatorDelegationSnapshotCodec(): Codec<
  VaultOperatorDelegationSnapshotArgs,
  VaultOperatorDelegationSnapshot
> {
  return combineCodec(
    getVaultOperatorDelegationSnapshotEncoder(),
    getVaultOperatorDelegationSnapshotDecoder()
  );
}

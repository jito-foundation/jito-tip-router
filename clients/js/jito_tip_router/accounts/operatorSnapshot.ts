/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  assertAccountExists,
  assertAccountsExist,
  combineCodec,
  decodeAccount,
  fetchEncodedAccount,
  fetchEncodedAccounts,
  getAddressDecoder,
  getAddressEncoder,
  getArrayDecoder,
  getArrayEncoder,
  getStructDecoder,
  getStructEncoder,
  getU128Decoder,
  getU128Encoder,
  getU16Decoder,
  getU16Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  type Account,
  type Address,
  type Codec,
  type Decoder,
  type EncodedAccount,
  type Encoder,
  type FetchAccountConfig,
  type FetchAccountsConfig,
  type MaybeAccount,
  type MaybeEncodedAccount,
} from '@solana/web3.js';
import {
  getVaultOperatorDelegationSnapshotDecoder,
  getVaultOperatorDelegationSnapshotEncoder,
  type VaultOperatorDelegationSnapshot,
  type VaultOperatorDelegationSnapshotArgs,
} from '../types';

export type OperatorSnapshot = {
  discriminator: bigint;
  operator: Address;
  ncn: Address;
  ncnEpoch: bigint;
  slotCreated: bigint;
  bump: number;
  operatorFeeBps: number;
  totalVotes: bigint;
  numVaultOperatorDelegations: number;
  vaultOperatorDelegationsRegistered: number;
  slotSet: bigint;
  vaultOperatorDelegations: Array<VaultOperatorDelegationSnapshot>;
  reserved: Array<number>;
};

export type OperatorSnapshotArgs = {
  discriminator: number | bigint;
  operator: Address;
  ncn: Address;
  ncnEpoch: number | bigint;
  slotCreated: number | bigint;
  bump: number;
  operatorFeeBps: number;
  totalVotes: number | bigint;
  numVaultOperatorDelegations: number;
  vaultOperatorDelegationsRegistered: number;
  slotSet: number | bigint;
  vaultOperatorDelegations: Array<VaultOperatorDelegationSnapshotArgs>;
  reserved: Array<number>;
};

export function getOperatorSnapshotEncoder(): Encoder<OperatorSnapshotArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['operator', getAddressEncoder()],
    ['ncn', getAddressEncoder()],
    ['ncnEpoch', getU64Encoder()],
    ['slotCreated', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['operatorFeeBps', getU16Encoder()],
    ['totalVotes', getU128Encoder()],
    ['numVaultOperatorDelegations', getU16Encoder()],
    ['vaultOperatorDelegationsRegistered', getU16Encoder()],
    ['slotSet', getU64Encoder()],
    [
      'vaultOperatorDelegations',
      getArrayEncoder(getVaultOperatorDelegationSnapshotEncoder(), {
        size: 32,
      }),
    ],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
  ]);
}

export function getOperatorSnapshotDecoder(): Decoder<OperatorSnapshot> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['operator', getAddressDecoder()],
    ['ncn', getAddressDecoder()],
    ['ncnEpoch', getU64Decoder()],
    ['slotCreated', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['operatorFeeBps', getU16Decoder()],
    ['totalVotes', getU128Decoder()],
    ['numVaultOperatorDelegations', getU16Decoder()],
    ['vaultOperatorDelegationsRegistered', getU16Decoder()],
    ['slotSet', getU64Decoder()],
    [
      'vaultOperatorDelegations',
      getArrayDecoder(getVaultOperatorDelegationSnapshotDecoder(), {
        size: 32,
      }),
    ],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
  ]);
}

export function getOperatorSnapshotCodec(): Codec<
  OperatorSnapshotArgs,
  OperatorSnapshot
> {
  return combineCodec(
    getOperatorSnapshotEncoder(),
    getOperatorSnapshotDecoder()
  );
}

export function decodeOperatorSnapshot<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<OperatorSnapshot, TAddress>;
export function decodeOperatorSnapshot<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<OperatorSnapshot, TAddress>;
export function decodeOperatorSnapshot<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<OperatorSnapshot, TAddress>
  | MaybeAccount<OperatorSnapshot, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getOperatorSnapshotDecoder()
  );
}

export async function fetchOperatorSnapshot<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<OperatorSnapshot, TAddress>> {
  const maybeAccount = await fetchMaybeOperatorSnapshot(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeOperatorSnapshot<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<OperatorSnapshot, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeOperatorSnapshot(maybeAccount);
}

export async function fetchAllOperatorSnapshot(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<OperatorSnapshot>[]> {
  const maybeAccounts = await fetchAllMaybeOperatorSnapshot(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeOperatorSnapshot(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<OperatorSnapshot>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeOperatorSnapshot(maybeAccount)
  );
}

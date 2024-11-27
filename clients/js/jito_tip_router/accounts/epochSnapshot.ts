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
  getFeesDecoder,
  getFeesEncoder,
  getStakeWeightDecoder,
  getStakeWeightEncoder,
  type Fees,
  type FeesArgs,
  type StakeWeight,
  type StakeWeightArgs,
} from '../types';

export type EpochSnapshot = {
  discriminator: bigint;
  ncn: Address;
  ncnEpoch: bigint;
  bump: number;
  slotCreated: bigint;
  slotFinalized: bigint;
  fees: Fees;
  operatorCount: bigint;
  vaultCount: bigint;
  operatorsRegistered: bigint;
  validOperatorVaultDelegations: bigint;
  stakeWeight: StakeWeight;
  reserved: Array<number>;
};

export type EpochSnapshotArgs = {
  discriminator: number | bigint;
  ncn: Address;
  ncnEpoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  slotFinalized: number | bigint;
  fees: FeesArgs;
  operatorCount: number | bigint;
  vaultCount: number | bigint;
  operatorsRegistered: number | bigint;
  validOperatorVaultDelegations: number | bigint;
  stakeWeight: StakeWeightArgs;
  reserved: Array<number>;
};

export function getEpochSnapshotEncoder(): Encoder<EpochSnapshotArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['ncnEpoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['slotFinalized', getU64Encoder()],
    ['fees', getFeesEncoder()],
    ['operatorCount', getU64Encoder()],
    ['vaultCount', getU64Encoder()],
    ['operatorsRegistered', getU64Encoder()],
    ['validOperatorVaultDelegations', getU64Encoder()],
    ['stakeWeight', getStakeWeightEncoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
  ]);
}

export function getEpochSnapshotDecoder(): Decoder<EpochSnapshot> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['ncnEpoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['slotFinalized', getU64Decoder()],
    ['fees', getFeesDecoder()],
    ['operatorCount', getU64Decoder()],
    ['vaultCount', getU64Decoder()],
    ['operatorsRegistered', getU64Decoder()],
    ['validOperatorVaultDelegations', getU64Decoder()],
    ['stakeWeight', getStakeWeightDecoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
  ]);
}

export function getEpochSnapshotCodec(): Codec<
  EpochSnapshotArgs,
  EpochSnapshot
> {
  return combineCodec(getEpochSnapshotEncoder(), getEpochSnapshotDecoder());
}

export function decodeEpochSnapshot<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<EpochSnapshot, TAddress>;
export function decodeEpochSnapshot<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<EpochSnapshot, TAddress>;
export function decodeEpochSnapshot<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
): Account<EpochSnapshot, TAddress> | MaybeAccount<EpochSnapshot, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getEpochSnapshotDecoder()
  );
}

export async function fetchEpochSnapshot<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<EpochSnapshot, TAddress>> {
  const maybeAccount = await fetchMaybeEpochSnapshot(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeEpochSnapshot<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<EpochSnapshot, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeEpochSnapshot(maybeAccount);
}

export async function fetchAllEpochSnapshot(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<EpochSnapshot>[]> {
  const maybeAccounts = await fetchAllMaybeEpochSnapshot(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeEpochSnapshot(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<EpochSnapshot>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) => decodeEpochSnapshot(maybeAccount));
}

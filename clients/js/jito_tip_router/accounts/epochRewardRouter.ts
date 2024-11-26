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
  getRewardBucketDecoder,
  getRewardBucketEncoder,
  getRewardRoutesDecoder,
  getRewardRoutesEncoder,
  type RewardBucket,
  type RewardBucketArgs,
  type RewardRoutes,
  type RewardRoutesArgs,
} from '../types';

export type EpochRewardRouter = {
  discriminator: bigint;
  ncn: Address;
  ncnEpoch: bigint;
  bump: number;
  slotCreated: bigint;
  rewardPool: bigint;
  doaRewards: bigint;
  reserved: Array<number>;
  ncnRewardBuckets: Array<RewardBucket>;
  rewardRoutes: Array<RewardRoutes>;
};

export type EpochRewardRouterArgs = {
  discriminator: number | bigint;
  ncn: Address;
  ncnEpoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  rewardPool: number | bigint;
  doaRewards: number | bigint;
  reserved: Array<number>;
  ncnRewardBuckets: Array<RewardBucketArgs>;
  rewardRoutes: Array<RewardRoutesArgs>;
};

export function getEpochRewardRouterEncoder(): Encoder<EpochRewardRouterArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['ncnEpoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['rewardPool', getU64Encoder()],
    ['doaRewards', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
    [
      'ncnRewardBuckets',
      getArrayEncoder(getRewardBucketEncoder(), { size: 8 }),
    ],
    ['rewardRoutes', getArrayEncoder(getRewardRoutesEncoder(), { size: 32 })],
  ]);
}

export function getEpochRewardRouterDecoder(): Decoder<EpochRewardRouter> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['ncnEpoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['rewardPool', getU64Decoder()],
    ['doaRewards', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
    [
      'ncnRewardBuckets',
      getArrayDecoder(getRewardBucketDecoder(), { size: 8 }),
    ],
    ['rewardRoutes', getArrayDecoder(getRewardRoutesDecoder(), { size: 32 })],
  ]);
}

export function getEpochRewardRouterCodec(): Codec<
  EpochRewardRouterArgs,
  EpochRewardRouter
> {
  return combineCodec(
    getEpochRewardRouterEncoder(),
    getEpochRewardRouterDecoder()
  );
}

export function decodeEpochRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<EpochRewardRouter, TAddress>;
export function decodeEpochRewardRouter<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<EpochRewardRouter, TAddress>;
export function decodeEpochRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<EpochRewardRouter, TAddress>
  | MaybeAccount<EpochRewardRouter, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getEpochRewardRouterDecoder()
  );
}

export async function fetchEpochRewardRouter<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<EpochRewardRouter, TAddress>> {
  const maybeAccount = await fetchMaybeEpochRewardRouter(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeEpochRewardRouter<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<EpochRewardRouter, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeEpochRewardRouter(maybeAccount);
}

export async function fetchAllEpochRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<EpochRewardRouter>[]> {
  const maybeAccounts = await fetchAllMaybeEpochRewardRouter(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeEpochRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<EpochRewardRouter>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeEpochRewardRouter(maybeAccount)
  );
}

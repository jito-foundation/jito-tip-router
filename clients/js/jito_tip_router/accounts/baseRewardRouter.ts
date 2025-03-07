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
  getBaseRewardRouterRewardsDecoder,
  getBaseRewardRouterRewardsEncoder,
  getNcnRewardRouteDecoder,
  getNcnRewardRouteEncoder,
  type BaseRewardRouterRewards,
  type BaseRewardRouterRewardsArgs,
  type NcnRewardRoute,
  type NcnRewardRouteArgs,
} from '../types';

export type BaseRewardRouter = {
  discriminator: bigint;
  ncn: Address;
  epoch: bigint;
  bump: number;
  slotCreated: bigint;
  totalRewards: bigint;
  rewardPool: bigint;
  rewardsProcessed: bigint;
  reserved: Array<number>;
  lastNcnGroupIndex: number;
  lastVoteIndex: number;
  lastRewardsToProcess: bigint;
  baseFeeGroupRewards: Array<BaseRewardRouterRewards>;
  ncnFeeGroupRewards: Array<BaseRewardRouterRewards>;
  ncnFeeGroupRewardRoutes: Array<NcnRewardRoute>;
};

export type BaseRewardRouterArgs = {
  discriminator: number | bigint;
  ncn: Address;
  epoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  totalRewards: number | bigint;
  rewardPool: number | bigint;
  rewardsProcessed: number | bigint;
  reserved: Array<number>;
  lastNcnGroupIndex: number;
  lastVoteIndex: number;
  lastRewardsToProcess: number | bigint;
  baseFeeGroupRewards: Array<BaseRewardRouterRewardsArgs>;
  ncnFeeGroupRewards: Array<BaseRewardRouterRewardsArgs>;
  ncnFeeGroupRewardRoutes: Array<NcnRewardRouteArgs>;
};

export function getBaseRewardRouterEncoder(): Encoder<BaseRewardRouterArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['epoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['totalRewards', getU64Encoder()],
    ['rewardPool', getU64Encoder()],
    ['rewardsProcessed', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
    ['lastNcnGroupIndex', getU8Encoder()],
    ['lastVoteIndex', getU16Encoder()],
    ['lastRewardsToProcess', getU64Encoder()],
    [
      'baseFeeGroupRewards',
      getArrayEncoder(getBaseRewardRouterRewardsEncoder(), { size: 8 }),
    ],
    [
      'ncnFeeGroupRewards',
      getArrayEncoder(getBaseRewardRouterRewardsEncoder(), { size: 8 }),
    ],
    [
      'ncnFeeGroupRewardRoutes',
      getArrayEncoder(getNcnRewardRouteEncoder(), { size: 256 }),
    ],
  ]);
}

export function getBaseRewardRouterDecoder(): Decoder<BaseRewardRouter> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['epoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['totalRewards', getU64Decoder()],
    ['rewardPool', getU64Decoder()],
    ['rewardsProcessed', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
    ['lastNcnGroupIndex', getU8Decoder()],
    ['lastVoteIndex', getU16Decoder()],
    ['lastRewardsToProcess', getU64Decoder()],
    [
      'baseFeeGroupRewards',
      getArrayDecoder(getBaseRewardRouterRewardsDecoder(), { size: 8 }),
    ],
    [
      'ncnFeeGroupRewards',
      getArrayDecoder(getBaseRewardRouterRewardsDecoder(), { size: 8 }),
    ],
    [
      'ncnFeeGroupRewardRoutes',
      getArrayDecoder(getNcnRewardRouteDecoder(), { size: 256 }),
    ],
  ]);
}

export function getBaseRewardRouterCodec(): Codec<
  BaseRewardRouterArgs,
  BaseRewardRouter
> {
  return combineCodec(
    getBaseRewardRouterEncoder(),
    getBaseRewardRouterDecoder()
  );
}

export function decodeBaseRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<BaseRewardRouter, TAddress>;
export function decodeBaseRewardRouter<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<BaseRewardRouter, TAddress>;
export function decodeBaseRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<BaseRewardRouter, TAddress>
  | MaybeAccount<BaseRewardRouter, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getBaseRewardRouterDecoder()
  );
}

export async function fetchBaseRewardRouter<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<BaseRewardRouter, TAddress>> {
  const maybeAccount = await fetchMaybeBaseRewardRouter(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeBaseRewardRouter<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<BaseRewardRouter, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeBaseRewardRouter(maybeAccount);
}

export async function fetchAllBaseRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<BaseRewardRouter>[]> {
  const maybeAccounts = await fetchAllMaybeBaseRewardRouter(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeBaseRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<BaseRewardRouter>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeBaseRewardRouter(maybeAccount)
  );
}

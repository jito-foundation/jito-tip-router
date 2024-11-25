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
  getVaultRewardDecoder,
  getVaultRewardEncoder,
  type VaultReward,
  type VaultRewardArgs,
} from '../types';

export type OperatorRewardRouter = {
  discriminator: bigint;
  ncn: Address;
  ncnEpoch: bigint;
  bump: number;
  slotCreated: bigint;
  reserved: Array<number>;
  rewardPool: bigint;
  vaultRewards: Array<VaultReward>;
};

export type OperatorRewardRouterArgs = {
  discriminator: number | bigint;
  ncn: Address;
  ncnEpoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  reserved: Array<number>;
  rewardPool: number | bigint;
  vaultRewards: Array<VaultRewardArgs>;
};

export function getOperatorRewardRouterEncoder(): Encoder<OperatorRewardRouterArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['ncnEpoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 128 })],
    ['rewardPool', getU64Encoder()],
    ['vaultRewards', getArrayEncoder(getVaultRewardEncoder(), { size: 32 })],
  ]);
}

export function getOperatorRewardRouterDecoder(): Decoder<OperatorRewardRouter> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['ncnEpoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 128 })],
    ['rewardPool', getU64Decoder()],
    ['vaultRewards', getArrayDecoder(getVaultRewardDecoder(), { size: 32 })],
  ]);
}

export function getOperatorRewardRouterCodec(): Codec<
  OperatorRewardRouterArgs,
  OperatorRewardRouter
> {
  return combineCodec(
    getOperatorRewardRouterEncoder(),
    getOperatorRewardRouterDecoder()
  );
}

export function decodeOperatorRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<OperatorRewardRouter, TAddress>;
export function decodeOperatorRewardRouter<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<OperatorRewardRouter, TAddress>;
export function decodeOperatorRewardRouter<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<OperatorRewardRouter, TAddress>
  | MaybeAccount<OperatorRewardRouter, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getOperatorRewardRouterDecoder()
  );
}

export async function fetchOperatorRewardRouter<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<OperatorRewardRouter, TAddress>> {
  const maybeAccount = await fetchMaybeOperatorRewardRouter(
    rpc,
    address,
    config
  );
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeOperatorRewardRouter<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<OperatorRewardRouter, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeOperatorRewardRouter(maybeAccount);
}

export async function fetchAllOperatorRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<OperatorRewardRouter>[]> {
  const maybeAccounts = await fetchAllMaybeOperatorRewardRouter(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeOperatorRewardRouter(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<OperatorRewardRouter>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeOperatorRewardRouter(maybeAccount)
  );
}

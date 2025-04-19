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
  getBoolDecoder,
  getBoolEncoder,
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
  getEpochAccountStatusDecoder,
  getEpochAccountStatusEncoder,
  getProgressDecoder,
  getProgressEncoder,
  type EpochAccountStatus,
  type EpochAccountStatusArgs,
  type Progress,
  type ProgressArgs,
} from '../types';

export type EpochState = {
  discriminator: bigint;
  ncn: Address;
  epoch: bigint;
  bump: number;
  slotCreated: bigint;
  wasTieBreakerSet: number;
  slotConsensusReached: bigint;
  operatorCount: bigint;
  vaultCount: bigint;
  accountStatus: EpochAccountStatus;
  setWeightProgress: Progress;
  epochSnapshotProgress: Progress;
  operatorSnapshotProgress: Array<Progress>;
  votingProgress: Progress;
  validationProgress: Progress;
  uploadProgress: Progress;
  reservedDistributionSpace: Array<number>;
  isClosing: number;
  reserved: Array<number>;
};

export type EpochStateArgs = {
  discriminator: number | bigint;
  ncn: Address;
  epoch: number | bigint;
  bump: number;
  slotCreated: number | bigint;
  wasTieBreakerSet: number;
  slotConsensusReached: number | bigint;
  operatorCount: number | bigint;
  vaultCount: number | bigint;
  accountStatus: EpochAccountStatusArgs;
  setWeightProgress: ProgressArgs;
  epochSnapshotProgress: ProgressArgs;
  operatorSnapshotProgress: Array<ProgressArgs>;
  votingProgress: ProgressArgs;
  validationProgress: ProgressArgs;
  uploadProgress: ProgressArgs;
  reservedDistributionSpace: Array<number>;
  isClosing: number;
  reserved: Array<number>;
};

export function getEpochStateEncoder(): Encoder<EpochStateArgs> {
  return getStructEncoder([
    ['discriminator', getU64Encoder()],
    ['ncn', getAddressEncoder()],
    ['epoch', getU64Encoder()],
    ['bump', getU8Encoder()],
    ['slotCreated', getU64Encoder()],
    ['wasTieBreakerSet', getBoolEncoder()],
    ['slotConsensusReached', getU64Encoder()],
    ['operatorCount', getU64Encoder()],
    ['vaultCount', getU64Encoder()],
    ['accountStatus', getEpochAccountStatusEncoder()],
    ['setWeightProgress', getProgressEncoder()],
    ['epochSnapshotProgress', getProgressEncoder()],
    [
      'operatorSnapshotProgress',
      getArrayEncoder(getProgressEncoder(), { size: 256 }),
    ],
    ['votingProgress', getProgressEncoder()],
    ['validationProgress', getProgressEncoder()],
    ['uploadProgress', getProgressEncoder()],
    [
      'reservedDistributionSpace',
      getArrayEncoder(getU8Encoder(), { size: 2064 }),
    ],
    ['isClosing', getBoolEncoder()],
    ['reserved', getArrayEncoder(getU8Encoder(), { size: 1023 })],
  ]);
}

export function getEpochStateDecoder(): Decoder<EpochState> {
  return getStructDecoder([
    ['discriminator', getU64Decoder()],
    ['ncn', getAddressDecoder()],
    ['epoch', getU64Decoder()],
    ['bump', getU8Decoder()],
    ['slotCreated', getU64Decoder()],
    ['wasTieBreakerSet', getBoolDecoder()],
    ['slotConsensusReached', getU64Decoder()],
    ['operatorCount', getU64Decoder()],
    ['vaultCount', getU64Decoder()],
    ['accountStatus', getEpochAccountStatusDecoder()],
    ['setWeightProgress', getProgressDecoder()],
    ['epochSnapshotProgress', getProgressDecoder()],
    [
      'operatorSnapshotProgress',
      getArrayDecoder(getProgressDecoder(), { size: 256 }),
    ],
    ['votingProgress', getProgressDecoder()],
    ['validationProgress', getProgressDecoder()],
    ['uploadProgress', getProgressDecoder()],
    [
      'reservedDistributionSpace',
      getArrayDecoder(getU8Decoder(), { size: 2064 }),
    ],
    ['isClosing', getBoolDecoder()],
    ['reserved', getArrayDecoder(getU8Decoder(), { size: 1023 })],
  ]);
}

export function getEpochStateCodec(): Codec<EpochStateArgs, EpochState> {
  return combineCodec(getEpochStateEncoder(), getEpochStateDecoder());
}

export function decodeEpochState<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<EpochState, TAddress>;
export function decodeEpochState<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<EpochState, TAddress>;
export function decodeEpochState<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
): Account<EpochState, TAddress> | MaybeAccount<EpochState, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getEpochStateDecoder()
  );
}

export async function fetchEpochState<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<EpochState, TAddress>> {
  const maybeAccount = await fetchMaybeEpochState(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeEpochState<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<EpochState, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeEpochState(maybeAccount);
}

export async function fetchAllEpochState(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<EpochState>[]> {
  const maybeAccounts = await fetchAllMaybeEpochState(rpc, addresses, config);
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeEpochState(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<EpochState>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) => decodeEpochState(maybeAccount));
}

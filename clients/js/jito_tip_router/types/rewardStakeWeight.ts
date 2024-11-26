/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU128Decoder,
  getU128Encoder,
  type Codec,
  type Decoder,
  type Encoder,
} from '@solana/web3.js';

export type RewardStakeWeight = { rewardStakeWeight: bigint };

export type RewardStakeWeightArgs = { rewardStakeWeight: number | bigint };

export function getRewardStakeWeightEncoder(): Encoder<RewardStakeWeightArgs> {
  return getStructEncoder([['rewardStakeWeight', getU128Encoder()]]);
}

export function getRewardStakeWeightDecoder(): Decoder<RewardStakeWeight> {
  return getStructDecoder([['rewardStakeWeight', getU128Decoder()]]);
}

export function getRewardStakeWeightCodec(): Codec<
  RewardStakeWeightArgs,
  RewardStakeWeight
> {
  return combineCodec(
    getRewardStakeWeightEncoder(),
    getRewardStakeWeightDecoder()
  );
}

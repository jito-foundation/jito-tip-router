/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  containsBytes,
  getU8Encoder,
  type Address,
  type ReadonlyUint8Array,
} from '@solana/web3.js';
import {
  type ParsedAdminRegisterStMintInstruction,
  type ParsedAdminSetConfigFeesInstruction,
  type ParsedAdminSetNewAdminInstruction,
  type ParsedAdminSetParametersInstruction,
  type ParsedAdminSetStMintInstruction,
  type ParsedAdminSetTieBreakerInstruction,
  type ParsedAdminSetWeightInstruction,
  type ParsedCastVoteInstruction,
  type ParsedClaimWithPayerInstruction,
  type ParsedCloseEpochAccountInstruction,
  type ParsedDistributeBaseNcnRewardRouteInstruction,
  type ParsedDistributeBaseRewardsInstruction,
  type ParsedDistributeNcnOperatorRewardsInstruction,
  type ParsedDistributeNcnVaultRewardsInstruction,
  type ParsedInitializeBallotBoxInstruction,
  type ParsedInitializeBaseRewardRouterInstruction,
  type ParsedInitializeConfigInstruction,
  type ParsedInitializeEpochSnapshotInstruction,
  type ParsedInitializeEpochStateInstruction,
  type ParsedInitializeNcnRewardRouterInstruction,
  type ParsedInitializeOperatorSnapshotInstruction,
  type ParsedInitializeVaultRegistryInstruction,
  type ParsedInitializeWeightTableInstruction,
  type ParsedReallocBallotBoxInstruction,
  type ParsedReallocBaseRewardRouterInstruction,
  type ParsedReallocEpochStateInstruction,
  type ParsedReallocOperatorSnapshotInstruction,
  type ParsedReallocVaultRegistryInstruction,
  type ParsedReallocWeightTableInstruction,
  type ParsedRegisterVaultInstruction,
  type ParsedRouteBaseRewardsInstruction,
  type ParsedRouteNcnRewardsInstruction,
  type ParsedSetMerkleRootInstruction,
  type ParsedSnapshotVaultOperatorDelegationInstruction,
  type ParsedSwitchboardSetWeightInstruction,
} from '../instructions';

export const JITO_TIP_ROUTER_PROGRAM_ADDRESS =
  'RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb' as Address<'RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb'>;

export enum JitoTipRouterAccount {
  BallotBox,
  BaseRewardRouter,
  Config,
  EpochMarker,
  EpochSnapshot,
  OperatorSnapshot,
  EpochState,
  NcnRewardRouter,
  VaultRegistry,
  WeightTable,
}

export enum JitoTipRouterInstruction {
  InitializeConfig,
  InitializeVaultRegistry,
  ReallocVaultRegistry,
  RegisterVault,
  InitializeEpochState,
  ReallocEpochState,
  InitializeWeightTable,
  ReallocWeightTable,
  SwitchboardSetWeight,
  InitializeEpochSnapshot,
  InitializeOperatorSnapshot,
  ReallocOperatorSnapshot,
  SnapshotVaultOperatorDelegation,
  InitializeBallotBox,
  ReallocBallotBox,
  CastVote,
  SetMerkleRoot,
  InitializeBaseRewardRouter,
  ReallocBaseRewardRouter,
  InitializeNcnRewardRouter,
  RouteBaseRewards,
  RouteNcnRewards,
  DistributeBaseRewards,
  DistributeBaseNcnRewardRoute,
  DistributeNcnOperatorRewards,
  DistributeNcnVaultRewards,
  ClaimWithPayer,
  CloseEpochAccount,
  AdminSetParameters,
  AdminSetConfigFees,
  AdminSetNewAdmin,
  AdminSetTieBreaker,
  AdminSetWeight,
  AdminRegisterStMint,
  AdminSetStMint,
}

export function identifyJitoTipRouterInstruction(
  instruction: { data: ReadonlyUint8Array } | ReadonlyUint8Array
): JitoTipRouterInstruction {
  const data = 'data' in instruction ? instruction.data : instruction;
  if (containsBytes(data, getU8Encoder().encode(0), 0)) {
    return JitoTipRouterInstruction.InitializeConfig;
  }
  if (containsBytes(data, getU8Encoder().encode(1), 0)) {
    return JitoTipRouterInstruction.InitializeVaultRegistry;
  }
  if (containsBytes(data, getU8Encoder().encode(2), 0)) {
    return JitoTipRouterInstruction.ReallocVaultRegistry;
  }
  if (containsBytes(data, getU8Encoder().encode(3), 0)) {
    return JitoTipRouterInstruction.RegisterVault;
  }
  if (containsBytes(data, getU8Encoder().encode(4), 0)) {
    return JitoTipRouterInstruction.InitializeEpochState;
  }
  if (containsBytes(data, getU8Encoder().encode(5), 0)) {
    return JitoTipRouterInstruction.ReallocEpochState;
  }
  if (containsBytes(data, getU8Encoder().encode(6), 0)) {
    return JitoTipRouterInstruction.InitializeWeightTable;
  }
  if (containsBytes(data, getU8Encoder().encode(7), 0)) {
    return JitoTipRouterInstruction.ReallocWeightTable;
  }
  if (containsBytes(data, getU8Encoder().encode(8), 0)) {
    return JitoTipRouterInstruction.SwitchboardSetWeight;
  }
  if (containsBytes(data, getU8Encoder().encode(9), 0)) {
    return JitoTipRouterInstruction.InitializeEpochSnapshot;
  }
  if (containsBytes(data, getU8Encoder().encode(10), 0)) {
    return JitoTipRouterInstruction.InitializeOperatorSnapshot;
  }
  if (containsBytes(data, getU8Encoder().encode(11), 0)) {
    return JitoTipRouterInstruction.ReallocOperatorSnapshot;
  }
  if (containsBytes(data, getU8Encoder().encode(12), 0)) {
    return JitoTipRouterInstruction.SnapshotVaultOperatorDelegation;
  }
  if (containsBytes(data, getU8Encoder().encode(13), 0)) {
    return JitoTipRouterInstruction.InitializeBallotBox;
  }
  if (containsBytes(data, getU8Encoder().encode(14), 0)) {
    return JitoTipRouterInstruction.ReallocBallotBox;
  }
  if (containsBytes(data, getU8Encoder().encode(15), 0)) {
    return JitoTipRouterInstruction.CastVote;
  }
  if (containsBytes(data, getU8Encoder().encode(16), 0)) {
    return JitoTipRouterInstruction.SetMerkleRoot;
  }
  if (containsBytes(data, getU8Encoder().encode(17), 0)) {
    return JitoTipRouterInstruction.InitializeBaseRewardRouter;
  }
  if (containsBytes(data, getU8Encoder().encode(18), 0)) {
    return JitoTipRouterInstruction.ReallocBaseRewardRouter;
  }
  if (containsBytes(data, getU8Encoder().encode(19), 0)) {
    return JitoTipRouterInstruction.InitializeNcnRewardRouter;
  }
  if (containsBytes(data, getU8Encoder().encode(20), 0)) {
    return JitoTipRouterInstruction.RouteBaseRewards;
  }
  if (containsBytes(data, getU8Encoder().encode(21), 0)) {
    return JitoTipRouterInstruction.RouteNcnRewards;
  }
  if (containsBytes(data, getU8Encoder().encode(22), 0)) {
    return JitoTipRouterInstruction.DistributeBaseRewards;
  }
  if (containsBytes(data, getU8Encoder().encode(23), 0)) {
    return JitoTipRouterInstruction.DistributeBaseNcnRewardRoute;
  }
  if (containsBytes(data, getU8Encoder().encode(24), 0)) {
    return JitoTipRouterInstruction.DistributeNcnOperatorRewards;
  }
  if (containsBytes(data, getU8Encoder().encode(25), 0)) {
    return JitoTipRouterInstruction.DistributeNcnVaultRewards;
  }
  if (containsBytes(data, getU8Encoder().encode(26), 0)) {
    return JitoTipRouterInstruction.ClaimWithPayer;
  }
  if (containsBytes(data, getU8Encoder().encode(27), 0)) {
    return JitoTipRouterInstruction.CloseEpochAccount;
  }
  if (containsBytes(data, getU8Encoder().encode(28), 0)) {
    return JitoTipRouterInstruction.AdminSetParameters;
  }
  if (containsBytes(data, getU8Encoder().encode(29), 0)) {
    return JitoTipRouterInstruction.AdminSetConfigFees;
  }
  if (containsBytes(data, getU8Encoder().encode(30), 0)) {
    return JitoTipRouterInstruction.AdminSetNewAdmin;
  }
  if (containsBytes(data, getU8Encoder().encode(31), 0)) {
    return JitoTipRouterInstruction.AdminSetTieBreaker;
  }
  if (containsBytes(data, getU8Encoder().encode(32), 0)) {
    return JitoTipRouterInstruction.AdminSetWeight;
  }
  if (containsBytes(data, getU8Encoder().encode(33), 0)) {
    return JitoTipRouterInstruction.AdminRegisterStMint;
  }
  if (containsBytes(data, getU8Encoder().encode(34), 0)) {
    return JitoTipRouterInstruction.AdminSetStMint;
  }
  throw new Error(
    'The provided instruction could not be identified as a jitoTipRouter instruction.'
  );
}

export type ParsedJitoTipRouterInstruction<
  TProgram extends string = 'RouterBmuRBkPUbgEDMtdvTZ75GBdSREZR5uGUxxxpb',
> =
  | ({
      instructionType: JitoTipRouterInstruction.InitializeConfig;
    } & ParsedInitializeConfigInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeVaultRegistry;
    } & ParsedInitializeVaultRegistryInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocVaultRegistry;
    } & ParsedReallocVaultRegistryInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.RegisterVault;
    } & ParsedRegisterVaultInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeEpochState;
    } & ParsedInitializeEpochStateInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocEpochState;
    } & ParsedReallocEpochStateInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeWeightTable;
    } & ParsedInitializeWeightTableInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocWeightTable;
    } & ParsedReallocWeightTableInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.SwitchboardSetWeight;
    } & ParsedSwitchboardSetWeightInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeEpochSnapshot;
    } & ParsedInitializeEpochSnapshotInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeOperatorSnapshot;
    } & ParsedInitializeOperatorSnapshotInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocOperatorSnapshot;
    } & ParsedReallocOperatorSnapshotInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.SnapshotVaultOperatorDelegation;
    } & ParsedSnapshotVaultOperatorDelegationInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeBallotBox;
    } & ParsedInitializeBallotBoxInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocBallotBox;
    } & ParsedReallocBallotBoxInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.CastVote;
    } & ParsedCastVoteInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.SetMerkleRoot;
    } & ParsedSetMerkleRootInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeBaseRewardRouter;
    } & ParsedInitializeBaseRewardRouterInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ReallocBaseRewardRouter;
    } & ParsedReallocBaseRewardRouterInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.InitializeNcnRewardRouter;
    } & ParsedInitializeNcnRewardRouterInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.RouteBaseRewards;
    } & ParsedRouteBaseRewardsInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.RouteNcnRewards;
    } & ParsedRouteNcnRewardsInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.DistributeBaseRewards;
    } & ParsedDistributeBaseRewardsInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.DistributeBaseNcnRewardRoute;
    } & ParsedDistributeBaseNcnRewardRouteInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.DistributeNcnOperatorRewards;
    } & ParsedDistributeNcnOperatorRewardsInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.DistributeNcnVaultRewards;
    } & ParsedDistributeNcnVaultRewardsInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.ClaimWithPayer;
    } & ParsedClaimWithPayerInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.CloseEpochAccount;
    } & ParsedCloseEpochAccountInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetParameters;
    } & ParsedAdminSetParametersInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetConfigFees;
    } & ParsedAdminSetConfigFeesInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetNewAdmin;
    } & ParsedAdminSetNewAdminInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetTieBreaker;
    } & ParsedAdminSetTieBreakerInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetWeight;
    } & ParsedAdminSetWeightInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminRegisterStMint;
    } & ParsedAdminRegisterStMintInstruction<TProgram>)
  | ({
      instructionType: JitoTipRouterInstruction.AdminSetStMint;
    } & ParsedAdminSetStMintInstruction<TProgram>);

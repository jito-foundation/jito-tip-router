/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  isProgramError,
  type Address,
  type SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM,
  type SolanaError,
} from '@solana/web3.js';
import { JITO_TIP_ROUTER_PROGRAM_ADDRESS } from '../programs';

/** DenominatorIsZero: Zero in the denominator */
export const JITO_TIP_ROUTER_ERROR__DENOMINATOR_IS_ZERO = 0x2100; // 8448
/** ArithmeticOverflow: Overflow */
export const JITO_TIP_ROUTER_ERROR__ARITHMETIC_OVERFLOW = 0x2101; // 8449
/** ArithmeticUnderflowError: Underflow */
export const JITO_TIP_ROUTER_ERROR__ARITHMETIC_UNDERFLOW_ERROR = 0x2102; // 8450
/** ArithmeticFloorError: Floor Overflow */
export const JITO_TIP_ROUTER_ERROR__ARITHMETIC_FLOOR_ERROR = 0x2103; // 8451
/** ModuloOverflow: Modulo Overflow */
export const JITO_TIP_ROUTER_ERROR__MODULO_OVERFLOW = 0x2104; // 8452
/** NewPreciseNumberError: New precise number error */
export const JITO_TIP_ROUTER_ERROR__NEW_PRECISE_NUMBER_ERROR = 0x2105; // 8453
/** CastToImpreciseNumberError: Cast to imprecise number error */
export const JITO_TIP_ROUTER_ERROR__CAST_TO_IMPRECISE_NUMBER_ERROR = 0x2106; // 8454
/** CastToU64Error: Cast to u64 error */
export const JITO_TIP_ROUTER_ERROR__CAST_TO_U64_ERROR = 0x2107; // 8455
/** IncorrectWeightTableAdmin: Incorrect weight table admin */
export const JITO_TIP_ROUTER_ERROR__INCORRECT_WEIGHT_TABLE_ADMIN = 0x2200; // 8704
/** DuplicateMintsInTable: Duplicate mints in table */
export const JITO_TIP_ROUTER_ERROR__DUPLICATE_MINTS_IN_TABLE = 0x2201; // 8705
/** NoMintsInTable: There are no mints in the table */
export const JITO_TIP_ROUTER_ERROR__NO_MINTS_IN_TABLE = 0x2202; // 8706
/** TooManyMintsForTable: Too many mints for table */
export const JITO_TIP_ROUTER_ERROR__TOO_MANY_MINTS_FOR_TABLE = 0x2203; // 8707
/** WeightTableAlreadyInitialized: Weight table already initialized */
export const JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_ALREADY_INITIALIZED = 0x2204; // 8708
/** CannotCreateFutureWeightTables: Cannnot create future weight tables */
export const JITO_TIP_ROUTER_ERROR__CANNOT_CREATE_FUTURE_WEIGHT_TABLES = 0x2205; // 8709
/** WeightMintsDoNotMatchLength: Weight mints do not match - length */
export const JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_LENGTH = 0x2206; // 8710
/** WeightMintsDoNotMatchMintHash: Weight mints do not match - mint hash */
export const JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_MINT_HASH = 0x2207; // 8711
/** InvalidMintForWeightTable: Invalid mint for weight table */
export const JITO_TIP_ROUTER_ERROR__INVALID_MINT_FOR_WEIGHT_TABLE = 0x2208; // 8712
/** ConfigMintsNotUpdated: Config supported mints do not match NCN Vault Count */
export const JITO_TIP_ROUTER_ERROR__CONFIG_MINTS_NOT_UPDATED = 0x2209; // 8713
/** ConfigMintListFull: NCN config vaults are at capacity */
export const JITO_TIP_ROUTER_ERROR__CONFIG_MINT_LIST_FULL = 0x220a; // 8714
/** TrackedMintListFull: Tracked mints are at capacity */
export const JITO_TIP_ROUTER_ERROR__TRACKED_MINT_LIST_FULL = 0x220b; // 8715
/** TrackedMintsLocked: Tracked mints are locked for the epoch */
export const JITO_TIP_ROUTER_ERROR__TRACKED_MINTS_LOCKED = 0x220c; // 8716
/** VaultIndexAlreadyInUse: Vault index already in use by a different mint */
export const JITO_TIP_ROUTER_ERROR__VAULT_INDEX_ALREADY_IN_USE = 0x220d; // 8717
/** MintEntryNotFound: Mint Entry not found */
export const JITO_TIP_ROUTER_ERROR__MINT_ENTRY_NOT_FOUND = 0x220e; // 8718
/** FeeCapExceeded: Fee cap exceeded */
export const JITO_TIP_ROUTER_ERROR__FEE_CAP_EXCEEDED = 0x220f; // 8719
/** IncorrectNcnAdmin: Incorrect NCN Admin */
export const JITO_TIP_ROUTER_ERROR__INCORRECT_NCN_ADMIN = 0x2210; // 8720
/** IncorrectNcn: Incorrect NCN */
export const JITO_TIP_ROUTER_ERROR__INCORRECT_NCN = 0x2211; // 8721
/** IncorrectFeeAdmin: Incorrect fee admin */
export const JITO_TIP_ROUTER_ERROR__INCORRECT_FEE_ADMIN = 0x2212; // 8722
/** WeightTableNotFinalized: Weight table not finalized */
export const JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_NOT_FINALIZED = 0x2213; // 8723
/** WeightNotFound: Weight not found */
export const JITO_TIP_ROUTER_ERROR__WEIGHT_NOT_FOUND = 0x2214; // 8724
/** NoOperators: No operators in ncn */
export const JITO_TIP_ROUTER_ERROR__NO_OPERATORS = 0x2215; // 8725
/** VaultOperatorDelegationFinalized: Vault operator delegation is already finalized - should not happen */
export const JITO_TIP_ROUTER_ERROR__VAULT_OPERATOR_DELEGATION_FINALIZED = 0x2216; // 8726
/** OperatorFinalized: Operator is already finalized - should not happen */
export const JITO_TIP_ROUTER_ERROR__OPERATOR_FINALIZED = 0x2217; // 8727
/** TooManyVaultOperatorDelegations: Too many vault operator delegations */
export const JITO_TIP_ROUTER_ERROR__TOO_MANY_VAULT_OPERATOR_DELEGATIONS = 0x2218; // 8728
/** DuplicateVaultOperatorDelegation: Duplicate vault operator delegation */
export const JITO_TIP_ROUTER_ERROR__DUPLICATE_VAULT_OPERATOR_DELEGATION = 0x2219; // 8729
/** DuplicateVoteCast: Duplicate Vote Cast */
export const JITO_TIP_ROUTER_ERROR__DUPLICATE_VOTE_CAST = 0x221a; // 8730
/** OperatorVotesFull: Operator votes full */
export const JITO_TIP_ROUTER_ERROR__OPERATOR_VOTES_FULL = 0x221b; // 8731
/** BallotTallyFull: Merkle root tally full */
export const JITO_TIP_ROUTER_ERROR__BALLOT_TALLY_FULL = 0x221c; // 8732
/** ConsensusAlreadyReached: Consensus already reached */
export const JITO_TIP_ROUTER_ERROR__CONSENSUS_ALREADY_REACHED = 0x221d; // 8733
/** ConsensusNotReached: Consensus not reached */
export const JITO_TIP_ROUTER_ERROR__CONSENSUS_NOT_REACHED = 0x221e; // 8734
/** InvalidNcnFeeGroup: Not a valid NCN fee group */
export const JITO_TIP_ROUTER_ERROR__INVALID_NCN_FEE_GROUP = 0x221f; // 8735
/** OperatorRewardListFull: Operator reward list full */
export const JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_LIST_FULL = 0x2220; // 8736
/** OperatorRewardNotFound: Operator Reward not found */
export const JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_NOT_FOUND = 0x2221; // 8737

export type JitoTipRouterError =
  | typeof JITO_TIP_ROUTER_ERROR__ARITHMETIC_FLOOR_ERROR
  | typeof JITO_TIP_ROUTER_ERROR__ARITHMETIC_OVERFLOW
  | typeof JITO_TIP_ROUTER_ERROR__ARITHMETIC_UNDERFLOW_ERROR
  | typeof JITO_TIP_ROUTER_ERROR__BALLOT_TALLY_FULL
  | typeof JITO_TIP_ROUTER_ERROR__CANNOT_CREATE_FUTURE_WEIGHT_TABLES
  | typeof JITO_TIP_ROUTER_ERROR__CAST_TO_IMPRECISE_NUMBER_ERROR
  | typeof JITO_TIP_ROUTER_ERROR__CAST_TO_U64_ERROR
  | typeof JITO_TIP_ROUTER_ERROR__CONFIG_MINT_LIST_FULL
  | typeof JITO_TIP_ROUTER_ERROR__CONFIG_MINTS_NOT_UPDATED
  | typeof JITO_TIP_ROUTER_ERROR__CONSENSUS_ALREADY_REACHED
  | typeof JITO_TIP_ROUTER_ERROR__CONSENSUS_NOT_REACHED
  | typeof JITO_TIP_ROUTER_ERROR__DENOMINATOR_IS_ZERO
  | typeof JITO_TIP_ROUTER_ERROR__DUPLICATE_MINTS_IN_TABLE
  | typeof JITO_TIP_ROUTER_ERROR__DUPLICATE_VAULT_OPERATOR_DELEGATION
  | typeof JITO_TIP_ROUTER_ERROR__DUPLICATE_VOTE_CAST
  | typeof JITO_TIP_ROUTER_ERROR__FEE_CAP_EXCEEDED
  | typeof JITO_TIP_ROUTER_ERROR__INCORRECT_FEE_ADMIN
  | typeof JITO_TIP_ROUTER_ERROR__INCORRECT_NCN
  | typeof JITO_TIP_ROUTER_ERROR__INCORRECT_NCN_ADMIN
  | typeof JITO_TIP_ROUTER_ERROR__INCORRECT_WEIGHT_TABLE_ADMIN
  | typeof JITO_TIP_ROUTER_ERROR__INVALID_MINT_FOR_WEIGHT_TABLE
  | typeof JITO_TIP_ROUTER_ERROR__INVALID_NCN_FEE_GROUP
  | typeof JITO_TIP_ROUTER_ERROR__MINT_ENTRY_NOT_FOUND
  | typeof JITO_TIP_ROUTER_ERROR__MODULO_OVERFLOW
  | typeof JITO_TIP_ROUTER_ERROR__NEW_PRECISE_NUMBER_ERROR
  | typeof JITO_TIP_ROUTER_ERROR__NO_MINTS_IN_TABLE
  | typeof JITO_TIP_ROUTER_ERROR__NO_OPERATORS
  | typeof JITO_TIP_ROUTER_ERROR__OPERATOR_FINALIZED
  | typeof JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_LIST_FULL
  | typeof JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_NOT_FOUND
  | typeof JITO_TIP_ROUTER_ERROR__OPERATOR_VOTES_FULL
  | typeof JITO_TIP_ROUTER_ERROR__TOO_MANY_MINTS_FOR_TABLE
  | typeof JITO_TIP_ROUTER_ERROR__TOO_MANY_VAULT_OPERATOR_DELEGATIONS
  | typeof JITO_TIP_ROUTER_ERROR__TRACKED_MINT_LIST_FULL
  | typeof JITO_TIP_ROUTER_ERROR__TRACKED_MINTS_LOCKED
  | typeof JITO_TIP_ROUTER_ERROR__VAULT_INDEX_ALREADY_IN_USE
  | typeof JITO_TIP_ROUTER_ERROR__VAULT_OPERATOR_DELEGATION_FINALIZED
  | typeof JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_LENGTH
  | typeof JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_MINT_HASH
  | typeof JITO_TIP_ROUTER_ERROR__WEIGHT_NOT_FOUND
  | typeof JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_ALREADY_INITIALIZED
  | typeof JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_NOT_FINALIZED;

let jitoTipRouterErrorMessages: Record<JitoTipRouterError, string> | undefined;
if (process.env.NODE_ENV !== 'production') {
  jitoTipRouterErrorMessages = {
    [JITO_TIP_ROUTER_ERROR__ARITHMETIC_FLOOR_ERROR]: `Floor Overflow`,
    [JITO_TIP_ROUTER_ERROR__ARITHMETIC_OVERFLOW]: `Overflow`,
    [JITO_TIP_ROUTER_ERROR__ARITHMETIC_UNDERFLOW_ERROR]: `Underflow`,
    [JITO_TIP_ROUTER_ERROR__BALLOT_TALLY_FULL]: `Merkle root tally full`,
    [JITO_TIP_ROUTER_ERROR__CANNOT_CREATE_FUTURE_WEIGHT_TABLES]: `Cannnot create future weight tables`,
    [JITO_TIP_ROUTER_ERROR__CAST_TO_IMPRECISE_NUMBER_ERROR]: `Cast to imprecise number error`,
    [JITO_TIP_ROUTER_ERROR__CAST_TO_U64_ERROR]: `Cast to u64 error`,
    [JITO_TIP_ROUTER_ERROR__CONFIG_MINT_LIST_FULL]: `NCN config vaults are at capacity`,
    [JITO_TIP_ROUTER_ERROR__CONFIG_MINTS_NOT_UPDATED]: `Config supported mints do not match NCN Vault Count`,
    [JITO_TIP_ROUTER_ERROR__CONSENSUS_ALREADY_REACHED]: `Consensus already reached`,
    [JITO_TIP_ROUTER_ERROR__CONSENSUS_NOT_REACHED]: `Consensus not reached`,
    [JITO_TIP_ROUTER_ERROR__DENOMINATOR_IS_ZERO]: `Zero in the denominator`,
    [JITO_TIP_ROUTER_ERROR__DUPLICATE_MINTS_IN_TABLE]: `Duplicate mints in table`,
    [JITO_TIP_ROUTER_ERROR__DUPLICATE_VAULT_OPERATOR_DELEGATION]: `Duplicate vault operator delegation`,
    [JITO_TIP_ROUTER_ERROR__DUPLICATE_VOTE_CAST]: `Duplicate Vote Cast`,
    [JITO_TIP_ROUTER_ERROR__FEE_CAP_EXCEEDED]: `Fee cap exceeded`,
    [JITO_TIP_ROUTER_ERROR__INCORRECT_FEE_ADMIN]: `Incorrect fee admin`,
    [JITO_TIP_ROUTER_ERROR__INCORRECT_NCN]: `Incorrect NCN`,
    [JITO_TIP_ROUTER_ERROR__INCORRECT_NCN_ADMIN]: `Incorrect NCN Admin`,
    [JITO_TIP_ROUTER_ERROR__INCORRECT_WEIGHT_TABLE_ADMIN]: `Incorrect weight table admin`,
    [JITO_TIP_ROUTER_ERROR__INVALID_MINT_FOR_WEIGHT_TABLE]: `Invalid mint for weight table`,
    [JITO_TIP_ROUTER_ERROR__INVALID_NCN_FEE_GROUP]: `Not a valid NCN fee group`,
    [JITO_TIP_ROUTER_ERROR__MINT_ENTRY_NOT_FOUND]: `Mint Entry not found`,
    [JITO_TIP_ROUTER_ERROR__MODULO_OVERFLOW]: `Modulo Overflow`,
    [JITO_TIP_ROUTER_ERROR__NEW_PRECISE_NUMBER_ERROR]: `New precise number error`,
    [JITO_TIP_ROUTER_ERROR__NO_MINTS_IN_TABLE]: `There are no mints in the table`,
    [JITO_TIP_ROUTER_ERROR__NO_OPERATORS]: `No operators in ncn`,
    [JITO_TIP_ROUTER_ERROR__OPERATOR_FINALIZED]: `Operator is already finalized - should not happen`,
    [JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_LIST_FULL]: `Operator reward list full`,
    [JITO_TIP_ROUTER_ERROR__OPERATOR_REWARD_NOT_FOUND]: `Operator Reward not found`,
    [JITO_TIP_ROUTER_ERROR__OPERATOR_VOTES_FULL]: `Operator votes full`,
    [JITO_TIP_ROUTER_ERROR__TOO_MANY_MINTS_FOR_TABLE]: `Too many mints for table`,
    [JITO_TIP_ROUTER_ERROR__TOO_MANY_VAULT_OPERATOR_DELEGATIONS]: `Too many vault operator delegations`,
    [JITO_TIP_ROUTER_ERROR__TRACKED_MINT_LIST_FULL]: `Tracked mints are at capacity`,
    [JITO_TIP_ROUTER_ERROR__TRACKED_MINTS_LOCKED]: `Tracked mints are locked for the epoch`,
    [JITO_TIP_ROUTER_ERROR__VAULT_INDEX_ALREADY_IN_USE]: `Vault index already in use by a different mint`,
    [JITO_TIP_ROUTER_ERROR__VAULT_OPERATOR_DELEGATION_FINALIZED]: `Vault operator delegation is already finalized - should not happen`,
    [JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_LENGTH]: `Weight mints do not match - length`,
    [JITO_TIP_ROUTER_ERROR__WEIGHT_MINTS_DO_NOT_MATCH_MINT_HASH]: `Weight mints do not match - mint hash`,
    [JITO_TIP_ROUTER_ERROR__WEIGHT_NOT_FOUND]: `Weight not found`,
    [JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_ALREADY_INITIALIZED]: `Weight table already initialized`,
    [JITO_TIP_ROUTER_ERROR__WEIGHT_TABLE_NOT_FINALIZED]: `Weight table not finalized`,
  };
}

export function getJitoTipRouterErrorMessage(code: JitoTipRouterError): string {
  if (process.env.NODE_ENV !== 'production') {
    return (jitoTipRouterErrorMessages as Record<JitoTipRouterError, string>)[
      code
    ];
  }

  return 'Error message not available in production bundles.';
}

export function isJitoTipRouterError<
  TProgramErrorCode extends JitoTipRouterError,
>(
  error: unknown,
  transactionMessage: {
    instructions: Record<number, { programAddress: Address }>;
  },
  code?: TProgramErrorCode
): error is SolanaError<typeof SOLANA_ERROR__INSTRUCTION_ERROR__CUSTOM> &
  Readonly<{ context: Readonly<{ code: TProgramErrorCode }> }> {
  return isProgramError<TProgramErrorCode>(
    error,
    transactionMessage,
    JITO_TIP_ROUTER_PROGRAM_ADDRESS,
    code
  );
}

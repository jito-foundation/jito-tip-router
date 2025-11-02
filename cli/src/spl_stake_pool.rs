use borsh::{BorshDeserialize, BorshSerialize};
use bytemuck::{Pod, Zeroable};
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use solana_program::pubkey::Pubkey;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::{Pack, Sealed},
};
/// Seed for withdraw authority seed
const AUTHORITY_WITHDRAW: &[u8] = b"withdraw";

/// Generates the withdraw authority program address for the stake pool
pub fn find_withdraw_authority_program_address(
    program_id: &Pubkey,
    stake_pool_address: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[stake_pool_address.as_ref(), AUTHORITY_WITHDRAW],
        program_id,
    )
}

#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Lockup {
    pub unix_timestamp: u64,
    pub epoch: u64,
    pub custodian: Pubkey,
}

/// Instructions supported by the `StakePool` program.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum StakePoolInstruction {
    ///   Initializes a new `StakePool`.
    ///
    ///   0. `[w]` New `StakePool` to create.
    ///   1. `[s]` Manager
    ///   2. `[]` Staker
    ///   3. `[]` Stake pool withdraw authority
    ///   4. `[w]` Uninitialized validator stake list storage account
    ///   5. `[]` Reserve stake account must be initialized, have zero balance,
    ///      and staker / withdrawer authority set to pool withdraw authority.
    ///   6. `[]` Pool token mint. Must have zero supply, owned by withdraw
    ///      authority.
    ///   7. `[]` Pool account to deposit the generated fee for manager.
    ///   8. `[]` Token program id
    ///   9. `[]` (Optional) Deposit authority that must sign all deposits.
    ///      Defaults to the program address generated using
    ///      `find_deposit_authority_program_address`, making deposits
    ///      permissionless.
    Initialize {
        /// Fee assessed as percentage of perceived rewards
        fee: Fee,
        /// Fee charged per withdrawal as percentage of withdrawal
        withdrawal_fee: Fee,
        /// Fee charged per deposit as percentage of deposit
        deposit_fee: Fee,
        /// Percentage [0-100] of `deposit_fee` that goes to referrer
        referral_fee: u8,
        /// Maximum expected number of validators
        max_validators: u32,
    },
    ///   Updates total pool balance based on balances in the reserve and
    ///   validator list
    ///
    ///   0. `[w]` Stake pool
    ///   1. `[]` Stake pool withdraw authority
    ///   2. `[w]` Validator stake list storage account
    ///   3. `[]` Reserve stake account
    ///   4. `[w]` Account to receive pool fee tokens
    ///   5. `[w]` Pool mint account
    ///   6. `[]` Pool token program
    UpdateStakePoolBalance,
}

/// Creates an `Initialize` instruction.
#[allow(clippy::too_many_arguments)]
pub fn initialize(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    manager: &Pubkey,
    staker: &Pubkey,
    stake_pool_withdraw_authority: &Pubkey,
    validator_list: &Pubkey,
    reserve_stake: &Pubkey,
    pool_mint: &Pubkey,
    manager_pool_account: &Pubkey,
    token_program_id: &Pubkey,
    deposit_authority: Option<Pubkey>,
    fee: Fee,
    withdrawal_fee: Fee,
    deposit_fee: Fee,
    referral_fee: u8,
    max_validators: u32,
) -> Instruction {
    let init_data = StakePoolInstruction::Initialize {
        fee,
        withdrawal_fee,
        deposit_fee,
        referral_fee,
        max_validators,
    };
    let data = borsh::to_vec(&init_data).unwrap();
    let mut accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*manager, true),
        AccountMeta::new_readonly(*staker, false),
        AccountMeta::new_readonly(*stake_pool_withdraw_authority, false),
        AccountMeta::new(*validator_list, false),
        AccountMeta::new_readonly(*reserve_stake, false),
        AccountMeta::new(*pool_mint, false),
        AccountMeta::new(*manager_pool_account, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    if let Some(deposit_authority) = deposit_authority {
        accounts.push(AccountMeta::new_readonly(deposit_authority, true));
    }
    Instruction {
        program_id: *program_id,
        accounts,
        data,
    }
}

/// Creates `UpdateStakePoolBalance` instruction (pool balance from the stake
/// account list balances)
#[allow(clippy::too_many_arguments)]
pub fn update_stake_pool_balance(
    program_id: &Pubkey,
    stake_pool: &Pubkey,
    withdraw_authority: &Pubkey,
    validator_list_storage: &Pubkey,
    reserve_stake: &Pubkey,
    manager_fee_account: &Pubkey,
    stake_pool_mint: &Pubkey,
    token_program_id: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*stake_pool, false),
        AccountMeta::new_readonly(*withdraw_authority, false),
        AccountMeta::new(*validator_list_storage, false),
        AccountMeta::new_readonly(*reserve_stake, false),
        AccountMeta::new(*manager_fee_account, false),
        AccountMeta::new(*stake_pool_mint, false),
        AccountMeta::new_readonly(*token_program_id, false),
    ];
    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&StakePoolInstruction::UpdateStakePoolBalance).unwrap(),
    }
}

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    #[default]
    Uninitialized,
    /// Stake pool
    StakePool,
    /// Validator stake list
    ValidatorList,
}

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq, BorshDeserialize)]
pub struct StakePool {
    /// Account type, must be `StakePool` currently
    pub account_type: AccountType,

    /// Manager authority, allows for updating the staker, manager, and fee
    /// account
    pub manager: Pubkey,

    /// Staker authority, allows for adding and removing validators, and
    /// managing stake distribution
    pub staker: Pubkey,

    /// Stake deposit authority
    ///
    /// If a depositor pubkey is specified on initialization, then deposits must
    /// be signed by this authority. If no deposit authority is specified,
    /// then the stake pool will default to the result of:
    /// `Pubkey::find_program_address(
    ///     &[&stake_pool_address.as_ref(), b"deposit"],
    ///     program_id,
    /// )`
    pub stake_deposit_authority: Pubkey,

    /// Stake withdrawal authority bump seed
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub stake_withdraw_bump_seed: u8,

    /// Validator stake list storage account
    pub validator_list: Pubkey,

    /// Reserve stake account, holds deactivated stake
    pub reserve_stake: Pubkey,

    /// Pool Mint
    pub pool_mint: Pubkey,

    /// Manager fee account
    pub manager_fee_account: Pubkey,

    /// Pool token program id
    pub token_program_id: Pubkey,

    /// Total stake under management.
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub total_lamports: u64,

    /// Total supply of pool tokens (should always match the supply in the Pool
    /// Mint)
    pub pool_token_supply: u64,

    /// Last epoch the `total_lamports` field was updated
    pub last_update_epoch: u64,

    /// Lockup that all stakes in the pool must have
    pub lockup: Lockup,

    /// Fee taken as a proportion of rewards each epoch
    pub epoch_fee: Fee,

    /// Fee for next epoch
    pub next_epoch_fee: FutureEpoch<Fee>,

    /// Preferred deposit validator vote account pubkey
    pub preferred_deposit_validator_vote_address: Option<Pubkey>,

    /// Preferred withdraw validator vote account pubkey
    pub preferred_withdraw_validator_vote_address: Option<Pubkey>,

    /// Fee assessed on stake deposits
    pub stake_deposit_fee: Fee,

    /// Fee assessed on withdrawals
    pub stake_withdrawal_fee: Fee,

    /// Future stake withdrawal fee, to be set for the following epoch
    pub next_stake_withdrawal_fee: FutureEpoch<Fee>,

    /// Fees paid out to referrers on referred stake deposits.
    /// Expressed as a percentage (0 - 100) of deposit fees.
    /// i.e. `stake_deposit_fee`% of stake deposited is collected as deposit
    /// fees for every deposit and `stake_referral_fee`% of the collected
    /// stake deposit fees is paid out to the referrer
    pub stake_referral_fee: u8,

    /// Toggles whether the `DepositSol` instruction requires a signature from
    /// this `sol_deposit_authority`
    pub sol_deposit_authority: Option<Pubkey>,

    /// Fee assessed on SOL deposits
    pub sol_deposit_fee: Fee,

    /// Fees paid out to referrers on referred SOL deposits.
    /// Expressed as a percentage (0 - 100) of SOL deposit fees.
    /// i.e. `sol_deposit_fee`% of SOL deposited is collected as deposit fees
    /// for every deposit and `sol_referral_fee`% of the collected SOL
    /// deposit fees is paid out to the referrer
    pub sol_referral_fee: u8,

    /// Toggles whether the `WithdrawSol` instruction requires a signature from
    /// the `deposit_authority`
    pub sol_withdraw_authority: Option<Pubkey>,

    /// Fee assessed on SOL withdrawals
    pub sol_withdrawal_fee: Fee,

    /// Future SOL withdrawal fee, to be set for the following epoch
    pub next_sol_withdrawal_fee: FutureEpoch<Fee>,

    /// Last epoch's total pool tokens, used only for APR estimation
    pub last_epoch_pool_token_supply: u64,

    /// Last epoch's total lamports, used only for APR estimation
    pub last_epoch_total_lamports: u64,
}

/// Storage list for all validator stake accounts in the pool.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize)]
pub struct ValidatorList {
    /// Data outside of the validator list, separated out for cheaper
    /// deserialization
    pub header: ValidatorListHeader,

    /// List of stake info for each validator in the pool
    pub validators: Vec<ValidatorStakeInfo>,
}

/// Helper type to deserialize just the start of a `ValidatorList`
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, Eq, BorshSerialize)]
pub struct ValidatorListHeader {
    /// Account type, must be `ValidatorList` currently
    pub account_type: AccountType,

    /// Maximum allowable number of validators
    pub max_validators: u32,
}

/// Status of the stake account in the validator list, for accounting
#[derive(Copy, Clone, Debug, PartialEq, Eq, FromPrimitive, ToPrimitive)]
pub enum StakeStatus {
    /// Stake account is active, there may be a transient stake as well
    Active,
    /// Only transient stake account exists, when a transient stake is
    /// deactivating during validator removal
    DeactivatingTransient,
    /// No more validator stake accounts exist, entry ready for removal during
    /// `UpdateStakePoolBalance`
    ReadyForRemoval,
    /// Only the validator stake account is deactivating, no transient stake
    /// account exists
    DeactivatingValidator,
    /// Both the transient and validator stake account are deactivating, when
    /// a validator is removed with a transient stake active
    DeactivatingAll,
}
impl Default for StakeStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Wrapper struct that can be `Pod`, containing a byte that *should* be a valid
/// `StakeStatus` underneath.
#[repr(transparent)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Pod, Zeroable, BorshSerialize, BorshDeserialize,
)]
pub struct PodStakeStatus(u8);
impl PodStakeStatus {
    /// Downgrade the status towards ready for removal by removing the validator
    /// stake
    pub fn remove_validator_stake(&mut self) -> Result<(), ProgramError> {
        let status = StakeStatus::try_from(*self)?;
        let new_self = match status {
            StakeStatus::Active
            | StakeStatus::DeactivatingTransient
            | StakeStatus::ReadyForRemoval => status,
            StakeStatus::DeactivatingAll => StakeStatus::DeactivatingTransient,
            StakeStatus::DeactivatingValidator => StakeStatus::ReadyForRemoval,
        };
        *self = new_self.into();
        Ok(())
    }
    /// Downgrade the status towards ready for removal by removing the transient
    /// stake
    pub fn remove_transient_stake(&mut self) -> Result<(), ProgramError> {
        let status = StakeStatus::try_from(*self)?;
        let new_self = match status {
            StakeStatus::Active
            | StakeStatus::DeactivatingValidator
            | StakeStatus::ReadyForRemoval => status,
            StakeStatus::DeactivatingAll => StakeStatus::DeactivatingValidator,
            StakeStatus::DeactivatingTransient => StakeStatus::ReadyForRemoval,
        };
        *self = new_self.into();
        Ok(())
    }
}
impl TryFrom<PodStakeStatus> for StakeStatus {
    type Error = ProgramError;
    fn try_from(pod: PodStakeStatus) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(pod.0).ok_or(ProgramError::InvalidAccountData)
    }
}

// Ignoring this because it is copied verbatim from the spl-stake-pool crate
#[allow(clippy::fallible_impl_from)]
impl From<StakeStatus> for PodStakeStatus {
    fn from(status: StakeStatus) -> Self {
        // unwrap is safe here because the variants of `StakeStatus` fit very
        // comfortably within a `u8`
        Self(status.to_u8().unwrap())
    }
}

/// Information about a validator in the pool
///
/// NOTE: ORDER IS VERY IMPORTANT HERE, PLEASE DO NOT RE-ORDER THE FIELDS UNLESS
/// THERE'S AN EXTREMELY GOOD REASON.
///
/// To save on BPF instructions, the serialized bytes are reinterpreted with a
/// `bytemuck` transmute, which means that this structure cannot have any
/// undeclared alignment-padding in its representation.
#[repr(C)]
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Zeroable, BorshDeserialize, BorshSerialize,
)]
pub struct ValidatorStakeInfo {
    /// Amount of lamports on the validator stake account, including rent
    ///
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub active_stake_lamports: u64,

    /// Amount of transient stake delegated to this validator
    ///
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub transient_stake_lamports: u64,

    /// Last epoch the active and transient stake lamports fields were updated
    pub last_update_epoch: u64,

    /// Transient account seed suffix, used to derive the transient stake
    /// account address
    pub transient_seed_suffix: u64,

    /// Unused space, initially meant to specify the end of seed suffixes
    pub unused: u32,

    /// Validator account seed suffix
    pub validator_seed_suffix: u32, // really `Option<NonZeroU32>` so 0 is `None`

    /// Status of the validator stake account
    pub status: PodStakeStatus,

    /// Validator vote account address
    pub vote_account_address: Pubkey,
}

impl Sealed for ValidatorStakeInfo {}

impl Pack for ValidatorStakeInfo {
    const LEN: usize = 73;
    fn pack_into_slice(&self, data: &mut [u8]) {
        // Removing this unwrap would require changing from `Pack` to some other
        // trait or `bytemuck`, so it stays in for now
        borsh::to_writer(data, self).unwrap();
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::try_from_slice(src)?;
        Ok(unpacked)
    }
}

impl ValidatorList {
    /// Create an empty instance containing space for `max_validators` and
    /// preferred validator keys
    pub fn new(max_validators: u32) -> Self {
        Self {
            header: ValidatorListHeader {
                account_type: AccountType::ValidatorList,
                max_validators,
            },
            validators: vec![ValidatorStakeInfo::default(); max_validators as usize],
        }
    }
}

/// Wrapper type that "counts down" epochs, which is Borsh-compatible with the
/// native `Option`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, BorshDeserialize)]
pub enum FutureEpoch<T> {
    /// Nothing is set
    None,
    /// Value is ready after the next epoch boundary
    One(T),
    /// Value is ready after two epoch boundaries
    Two(T),
}
impl<T> Default for FutureEpoch<T> {
    fn default() -> Self {
        Self::None
    }
}
impl<T> FutureEpoch<T> {
    /// Create a new value to be unlocked in a two epochs
    pub const fn new(value: T) -> Self {
        Self::Two(value)
    }
}
impl<T: Clone> FutureEpoch<T> {
    /// Update the epoch, to be done after `get`ting the underlying value
    pub fn update_epoch(&mut self) {
        match self {
            Self::None => {}
            Self::One(_) => {
                // The value has waited its last epoch
                *self = Self::None;
            }
            // The value still has to wait one more epoch after this
            Self::Two(v) => {
                *self = Self::One(v.clone());
            }
        }
    }

    /// Get the value if it's ready, which is only at `One` epoch remaining
    pub const fn get(&self) -> Option<&T> {
        match self {
            Self::None | Self::Two(_) => None,
            Self::One(v) => Some(v),
        }
    }
}
impl<T> From<FutureEpoch<T>> for Option<T> {
    fn from(v: FutureEpoch<T>) -> Self {
        match v {
            FutureEpoch::None => None,
            FutureEpoch::One(inner) | FutureEpoch::Two(inner) => Some(inner),
        }
    }
}

/// Fee rate as a ratio, minted on `UpdateStakePoolBalance` as a proportion of
/// the rewards
/// If either the numerator or the denominator is 0, the fee is considered to be
/// 0
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct Fee {
    /// denominator of the fee ratio
    pub denominator: u64,
    /// numerator of the fee ratio
    pub numerator: u64,
}

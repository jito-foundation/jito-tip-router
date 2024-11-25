use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::PodU64;
use shank::ShankType;
use solana_program::pubkey::Pubkey;
use spl_math::precise_number::PreciseNumber;

use crate::{constants::MAX_FEE_BPS, error::TipRouterError};

/// Fee Config. Allows for fee updates to take place in a future epoch without requiring an update.
/// This is important so all operators calculate the same Merkle root regardless of when fee changes take place.
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct FeeConfig {
    dao_fee_wallet: Pubkey,

    fee_1: Fees,
    fee_2: Fees,
}

impl FeeConfig {
    pub fn new(
        dao_fee_wallet: Pubkey,
        block_engine_fee_bps: u64,
        dao_fee_bps: u64,
        default_ncn_fee_bps: u64,
        current_epoch: u64,
    ) -> Self {
        let fee = Fees::new(
            block_engine_fee_bps,
            dao_fee_bps,
            default_ncn_fee_bps,
            current_epoch,
        );
        Self {
            dao_fee_wallet,
            fee_1: fee,
            fee_2: fee,
        }
    }

    fn current_fees(&self, current_epoch: u64) -> &Fees {
        // If either fee is not yet active, return the other one
        if self.fee_1.activation_epoch() > current_epoch {
            return &self.fee_2;
        }
        if self.fee_2.activation_epoch() > current_epoch {
            return &self.fee_1;
        }

        // Otherwise return the one with higher activation epoch
        if self.fee_1.activation_epoch() >= self.fee_2.activation_epoch() {
            &self.fee_1
        } else {
            &self.fee_2
        }
    }

    pub fn check_fees_okay(&self, current_epoch: u64) -> Result<(), TipRouterError> {
        let _ = self.precise_block_engine_fee_bps(current_epoch)?;
        let _ = self.adjusted_precise_dao_fee_bps(current_epoch)?;

        let all_fee_groups = NcnFeeGroup::all_groups();

        for group in all_fee_groups.iter() {
            let _ = self.adjusted_precise_ncn_fee_bps(*group, current_epoch)?;
        }

        Ok(())
    }

    pub fn block_engine_fee_bps(&self, current_epoch: u64) -> u64 {
        let current_fees = self.current_fees(current_epoch);
        current_fees.block_engine_fee_bps()
    }

    pub fn precise_block_engine_fee_bps(
        &self,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);

        PreciseNumber::new(current_fees.block_engine_fee_bps() as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn dao_fee_bps(&self, current_epoch: u64) -> u64 {
        let fees = self.current_fees(current_epoch);
        fees.dao_fee_bps()
    }

    /// Calculate fee as a portion of remaining BPS after block engine fee
    /// new_fee = dao_fee_bps / ((10000 - block_engine_fee_bps) / 10000)
    /// = dao_fee_bps * 10000 / (10000 - block_engine_fee_bps)
    pub fn adjusted_dao_fee_bps(&self, current_epoch: u64) -> Result<u64, TipRouterError> {
        let fees = self.current_fees(current_epoch);

        self.adjusted_fee_bps(fees.dao_fee_bps(), current_epoch)
    }

    pub fn adjusted_precise_dao_fee_bps(
        &self,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let fees = self.current_fees(current_epoch);

        self.adjusted_precise_fee_bps(fees.dao_fee_bps(), current_epoch)
    }

    pub fn ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<u64, TipRouterError> {
        let fees = self.current_fees(current_epoch);
        fees.ncn_fee_bps(ncn_fee_group)
    }

    pub fn adjusted_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<u64, TipRouterError> {
        let fees = self.current_fees(current_epoch);

        let fee = fees.ncn_fee_bps(ncn_fee_group)?;

        self.adjusted_fee_bps(fee, current_epoch)
    }

    pub fn adjusted_precise_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let fees = self.current_fees(current_epoch);

        let fee = fees.ncn_fee_bps(ncn_fee_group)?;

        self.adjusted_precise_fee_bps(fee, current_epoch)
    }

    pub const fn fee_wallet(&self) -> Pubkey {
        self.dao_fee_wallet
    }

    fn adjusted_fee_bps(&self, fee: u64, current_epoch: u64) -> Result<u64, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);

        let remaining_bps = MAX_FEE_BPS
            .checked_sub(current_fees.block_engine_fee_bps())
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        fee.checked_mul(MAX_FEE_BPS)
            .and_then(|x| x.checked_div(remaining_bps))
            .ok_or(TipRouterError::DenominatorIsZero)
    }

    fn adjusted_precise_fee_bps(
        &self,
        fee: u64,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let fees = self.current_fees(current_epoch);

        let remaining_bps = MAX_FEE_BPS
            .checked_sub(fees.block_engine_fee_bps())
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let precise_remaining_bps = PreciseNumber::new(remaining_bps as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let adjusted_fee = fee
            .checked_mul(MAX_FEE_BPS)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let precise_adjusted_fee = PreciseNumber::new(adjusted_fee as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        precise_adjusted_fee
            .checked_div(&precise_remaining_bps)
            .ok_or(TipRouterError::DenominatorIsZero)
    }

    fn get_updatable_fee_mut(&mut self, current_epoch: u64) -> &mut Fees {
        // If either fee is scheduled for next epoch, return that one
        if self.fee_1.activation_epoch() > current_epoch {
            return &mut self.fee_1;
        }
        if self.fee_2.activation_epoch() > current_epoch {
            return &mut self.fee_2;
        }

        // Otherwise return the one with lower activation epoch
        if self.fee_1.activation_epoch() <= self.fee_2.activation_epoch() {
            &mut self.fee_1
        } else {
            &mut self.fee_2
        }
    }

    /// Updates the Fee Config
    /// Any option set to None will be ignored
    /// `new_wallet`` and `new_block_engine_fee_bps` will take effect immediately
    /// `new_ncn_fee_bps` will set the fee group specified in `new_ncn_fee_group`
    /// if no `new_ncn_fee_group` is specified, the default ncn group will be set
    pub fn update_fee_config(
        &mut self,
        new_wallet: Option<Pubkey>,
        new_block_engine_fee_bps: Option<u64>,
        new_dao_fee_bps: Option<u64>,
        new_ncn_fee_bps: Option<u64>,
        new_ncn_fee_group: Option<NcnFeeGroup>,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        // Set Wallet
        if let Some(new_wallet) = new_wallet {
            self.dao_fee_wallet = new_wallet;
        }

        // Set new block engine fee
        if let Some(new_block_engine_fee_bps) = new_block_engine_fee_bps {
            self.fee_1
                .set_block_engine_fee_bps(new_block_engine_fee_bps);
            self.fee_2
                .set_block_engine_fee_bps(new_block_engine_fee_bps);
        }

        // Change Fees
        {
            let current_fees = *self.current_fees(current_epoch);
            let new_fees = self.get_updatable_fee_mut(current_epoch);
            *new_fees = current_fees;

            if let Some(new_dao_fee_bps) = new_dao_fee_bps {
                if new_dao_fee_bps > MAX_FEE_BPS {
                    return Err(TipRouterError::FeeCapExceeded);
                }
                new_fees.set_dao_fee_bps(new_dao_fee_bps);
            }

            // If no fee group is set, use the default
            if let Some(new_ncn_fee_bps) = new_ncn_fee_bps {
                if new_ncn_fee_bps > MAX_FEE_BPS {
                    return Err(TipRouterError::FeeCapExceeded);
                }

                if let Some(new_ncn_fee_group) = new_ncn_fee_group {
                    new_fees.set_ncn_fee_bps(new_ncn_fee_group, new_ncn_fee_bps)?;
                } else {
                    new_fees.set_ncn_fee_bps(NcnFeeGroup::default(), new_ncn_fee_bps)?;
                }
            }

            let next_epoch = current_epoch
                .checked_add(1)
                .ok_or(TipRouterError::ArithmeticOverflow)?;

            new_fees.set_activation_epoch(next_epoch);

            self.check_fees_okay(next_epoch)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Fees {
    activation_epoch: PodU64,

    block_engine_fee_bps: PodU64,
    dao_fee_bps: PodU64,

    ncn_fee_groups_bps: [NcnFee; 8],

    // Reserves
    reserved: [u8; 64],
}

impl Fees {
    pub fn new(
        block_engine_fee_bps: u64,
        dao_fee_bps: u64,
        default_ncn_fee_bps: u64,
        epoch: u64,
    ) -> Self {
        Self {
            activation_epoch: PodU64::from(epoch),
            block_engine_fee_bps: PodU64::from(block_engine_fee_bps),
            dao_fee_bps: PodU64::from(dao_fee_bps),
            ncn_fee_groups_bps: [NcnFee::new(default_ncn_fee_bps); 8],
            reserved: [0; 64],
        }
    }

    pub fn activation_epoch(&self) -> u64 {
        self.activation_epoch.into()
    }

    pub fn block_engine_fee_bps(&self) -> u64 {
        self.block_engine_fee_bps.into()
    }

    pub fn dao_fee_bps(&self) -> u64 {
        self.dao_fee_bps.into()
    }

    pub fn ncn_fee_bps(&self, ncn_fee_group: NcnFeeGroup) -> Result<u64, TipRouterError> {
        let group = ncn_fee_group.group()?;

        match group {
            NcnFeeGroupType::Default => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::JTO => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved2 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved3 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved4 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved5 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved6 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
            NcnFeeGroupType::Reserved7 => Ok(self.ncn_fee_groups_bps[group as usize].fee()),
        }
    }

    fn set_activation_epoch(&mut self, value: u64) {
        self.activation_epoch = PodU64::from(value);
    }

    fn set_block_engine_fee_bps(&mut self, value: u64) {
        self.block_engine_fee_bps = PodU64::from(value);
    }

    fn set_dao_fee_bps(&mut self, value: u64) {
        self.dao_fee_bps = PodU64::from(value);
    }

    pub fn set_ncn_fee_bps(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        value: u64,
    ) -> Result<(), TipRouterError> {
        let group = ncn_fee_group.group()?;

        match group {
            NcnFeeGroupType::Default => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::JTO => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved2 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved3 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved4 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved5 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved6 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
            NcnFeeGroupType::Reserved7 => {
                self.ncn_fee_groups_bps[group as usize] = NcnFee::new(value);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnFee {
    fee: PodU64,
}

impl NcnFee {
    pub fn new(fee: u64) -> Self {
        Self {
            fee: PodU64::from(fee),
        }
    }

    pub fn fee(&self) -> u64 {
        self.fee.into()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NcnFeeGroupType {
    Default = 0,
    JTO = 1,
    Reserved2 = 2,
    Reserved3 = 3,
    Reserved4 = 4,
    Reserved5 = 5,
    Reserved6 = 6,
    Reserved7 = 7,
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct NcnFeeGroup {
    pub group: u8,
}

impl Default for NcnFeeGroup {
    fn default() -> Self {
        Self {
            group: NcnFeeGroupType::Default as u8,
        }
    }
}

impl NcnFeeGroup {
    pub const fn new(group: NcnFeeGroupType) -> Self {
        Self { group: group as u8 }
    }

    pub const fn from_u8(group: u8) -> Result<Self, TipRouterError> {
        match group {
            0 => Ok(Self::new(NcnFeeGroupType::Default)),
            1 => Ok(Self::new(NcnFeeGroupType::JTO)),
            2 => Ok(Self::new(NcnFeeGroupType::Reserved2)),
            3 => Ok(Self::new(NcnFeeGroupType::Reserved3)),
            4 => Ok(Self::new(NcnFeeGroupType::Reserved4)),
            5 => Ok(Self::new(NcnFeeGroupType::Reserved5)),
            6 => Ok(Self::new(NcnFeeGroupType::Reserved6)),
            7 => Ok(Self::new(NcnFeeGroupType::Reserved7)),
            _ => Err(TipRouterError::NotValidNcnShareGroup),
        }
    }

    pub const fn group(&self) -> Result<NcnFeeGroupType, TipRouterError> {
        match self.group {
            0 => Ok(NcnFeeGroupType::Default),
            1 => Ok(NcnFeeGroupType::JTO),
            2 => Ok(NcnFeeGroupType::Reserved2),
            3 => Ok(NcnFeeGroupType::Reserved3),
            4 => Ok(NcnFeeGroupType::Reserved4),
            5 => Ok(NcnFeeGroupType::Reserved5),
            6 => Ok(NcnFeeGroupType::Reserved6),
            7 => Ok(NcnFeeGroupType::Reserved7),
            _ => Err(TipRouterError::NotValidNcnShareGroup),
        }
    }

    pub fn all_groups() -> Vec<Self> {
        vec![
            Self::new(NcnFeeGroupType::Default),
            Self::new(NcnFeeGroupType::JTO),
            Self::new(NcnFeeGroupType::Reserved2),
            Self::new(NcnFeeGroupType::Reserved3),
            Self::new(NcnFeeGroupType::Reserved4),
            Self::new(NcnFeeGroupType::Reserved5),
            Self::new(NcnFeeGroupType::Reserved6),
            Self::new(NcnFeeGroupType::Reserved7),
        ]
    }
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_update_fees() {
        const BLOCK_ENGINE_FEE: u64 = 100;
        const DAO_FEE: u64 = 200;
        const DEFAULT_NCN_FEE: u64 = 300;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();

        let mut fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        );

        assert_eq!(fee_config.fee_wallet(), dao_fee_wallet);

        assert_eq!(fee_config.fee_1.activation_epoch(), STARTING_EPOCH);
        assert_eq!(fee_config.fee_1.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
        assert_eq!(fee_config.fee_1.dao_fee_bps(), DAO_FEE);
        assert_eq!(
            fee_config
                .fee_1
                .ncn_fee_bps(NcnFeeGroup::default())
                .unwrap(),
            DEFAULT_NCN_FEE
        );

        assert_eq!(fee_config.fee_2.activation_epoch(), STARTING_EPOCH);
        assert_eq!(fee_config.fee_2.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
        assert_eq!(fee_config.fee_2.dao_fee_bps(), DAO_FEE);
        assert_eq!(
            fee_config
                .fee_2
                .ncn_fee_bps(NcnFeeGroup::default())
                .unwrap(),
            DEFAULT_NCN_FEE
        );

        let new_fees = Fees::new(500, 600, 700, 10);
        let new_wallet = Pubkey::new_unique();

        fee_config
            .update_fee_config(
                Some(new_wallet),
                Some(new_fees.block_engine_fee_bps()),
                Some(new_fees.dao_fee_bps()),
                Some(new_fees.ncn_fee_bps(NcnFeeGroup::default()).unwrap()),
                None,
                STARTING_EPOCH,
            )
            .unwrap();

        assert_eq!(fee_config.fee_wallet(), new_wallet);

        assert_eq!(fee_config.fee_1.activation_epoch(), STARTING_EPOCH + 1);
        assert_eq!(fee_config.fee_1.block_engine_fee_bps(), 500);
        assert_eq!(fee_config.fee_1.dao_fee_bps(), 600);
        assert_eq!(
            fee_config
                .fee_1
                .ncn_fee_bps(NcnFeeGroup::default())
                .unwrap(),
            700
        );

        assert_eq!(fee_config.fee_2.activation_epoch(), STARTING_EPOCH);
        assert_eq!(fee_config.fee_2.block_engine_fee_bps(), 500); // This will change regardless
        assert_eq!(fee_config.fee_2.dao_fee_bps(), DAO_FEE);
        assert_eq!(
            fee_config
                .fee_2
                .ncn_fee_bps(NcnFeeGroup::default())
                .unwrap(),
            DEFAULT_NCN_FEE
        );
    }

    // #[test]
    // fn test_update_fees() {
    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);
    //     let new_wallet = Pubkey::new_unique();

    //     fees.set_new_fees(Some(400), None, None, Some(new_wallet), 10)
    //         .unwrap();
    //     assert_eq!(fees.fee_1.dao_share_bps(), 400);
    //     assert_eq!(fees.wallet, new_wallet);
    //     assert_eq!(fees.fee_1.activation_epoch(), 11);
    // }

    // #[test]
    // fn test_update_all_fees() {
    //     let mut fees = Fees::new(Pubkey::new_unique(), 0, 0, 0, 5);

    //     fees.set_new_fees(Some(100), Some(200), Some(300), None, 10)
    //         .unwrap();
    //     assert_eq!(fees.fee_1.dao_share_bps(), 100);
    //     assert_eq!(fees.fee_1.ncn_share_bps(), 200);
    //     assert_eq!(fees.block_engine_fee_bps(), 300);
    //     assert_eq!(fees.fee_1.activation_epoch(), 11);
    // }

    // #[test]
    // fn test_update_fees_no_changes() {
    //     const DAO_SHARE_FEE_BPS: u64 = 100;
    //     const NCN_SHARE_FEE_BPS: u64 = 100;
    //     const BLOCK_ENGINE_FEE: u64 = 100;
    //     const STARTING_EPOCH: u64 = 10;

    //     let wallet = Pubkey::new_unique();

    //     let mut fees = Fees::new(
    //         wallet,
    //         DAO_SHARE_FEE_BPS,
    //         NCN_SHARE_FEE_BPS,
    //         BLOCK_ENGINE_FEE,
    //         STARTING_EPOCH,
    //     );

    //     fees.set_new_fees(None, None, None, None, STARTING_EPOCH)
    //         .unwrap();
    //     assert_eq!(fees.fee_1.dao_share_bps(), DAO_SHARE_FEE_BPS);
    //     assert_eq!(fees.fee_1.ncn_share_bps(), NCN_SHARE_FEE_BPS);
    //     assert_eq!(fees.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
    //     assert_eq!(fees.wallet, wallet);
    //     assert_eq!(fees.fee_1.activation_epoch(), STARTING_EPOCH + 1);
    // }

    // #[test]
    // fn test_update_fees_errors() {
    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);

    //     assert_eq!(
    //         fees.set_new_fees(Some(10001), None, None, None, 10),
    //         Err(TipRouterError::FeeCapExceeded)
    //     );

    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);

    //     assert_eq!(
    //         fees.set_new_fees(None, None, None, None, u64::MAX),
    //         Err(TipRouterError::ArithmeticOverflow)
    //     );

    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);

    //     assert_eq!(
    //         fees.set_new_fees(None, None, Some(MAX_FEE_BPS), None, 10),
    //         Err(TipRouterError::FeeCapExceeded)
    //     );
    // }

    // #[test]
    // fn test_check_fees_okay() {
    //     let fees = Fees::new(Pubkey::new_unique(), 0, 0, 0, 5);

    //     fees.check_fees_okay(5).unwrap();

    //     let fees = Fees::new(Pubkey::new_unique(), 0, 0, MAX_FEE_BPS, 5);

    //     assert_eq!(
    //         fees.check_fees_okay(5),
    //         Err(TipRouterError::DenominatorIsZero)
    //     );
    // }

    // #[test]
    // fn test_current_fee() {
    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);

    //     assert_eq!(fees.current_fee(5).activation_epoch(), 5);

    //     fees.fee_1.set_activation_epoch(10);

    //     assert_eq!(fees.current_fee(5).activation_epoch(), 5);
    //     assert_eq!(fees.current_fee(10).activation_epoch(), 10);

    //     fees.fee_2.set_activation_epoch(15);

    //     assert_eq!(fees.current_fee(12).activation_epoch(), 10);
    //     assert_eq!(fees.current_fee(15).activation_epoch(), 15);
    // }

    // #[test]
    // fn test_get_updatable_fee_mut() {
    //     let mut fees = Fees::new(Pubkey::new_unique(), 100, 200, 300, 5);

    //     let fee = fees.get_updatable_fee_mut(10);
    //     fee.set_dao_share_bps(400);
    //     fee.set_activation_epoch(11);

    //     assert_eq!(fees.fee_1.dao_share_bps(), 400);
    //     assert_eq!(fees.fee_1.activation_epoch(), 11);

    //     fees.fee_2.set_activation_epoch(13);

    //     let fee = fees.get_updatable_fee_mut(12);
    //     fee.set_dao_share_bps(500);
    //     fee.set_activation_epoch(13);

    //     assert_eq!(fees.fee_2.dao_share_bps(), 500);
    //     assert_eq!(fees.fee_2.activation_epoch(), 13);

    //     assert_eq!(fees.get_updatable_fee_mut(u64::MAX).activation_epoch(), 11);
    // }
}

use bytemuck::{Pod, Zeroable};
use jito_bytemuck::types::{PodU16, PodU64};
use shank::ShankType;
use solana_program::pubkey::Pubkey;
use spl_math::precise_number::PreciseNumber;

use crate::{
    base_fee_group::BaseFeeGroup, constants::MAX_FEE_BPS, error::TipRouterError,
    ncn_fee_group::NcnFeeGroup,
};

/// Fee Config. Allows for fee updates to take place in a future epoch without requiring an update.
/// This is important so all operators calculate the same Merkle root regardless of when fee changes take place.
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct FeeConfig {
    /// Carbon Copy
    block_engine_fee_bps: PodU16,

    // Wallets
    base_fee_wallets: [Pubkey; 8],

    reserved: [u8; 128],

    fee_1: Fees,
    fee_2: Fees,
}

impl FeeConfig {
    pub fn new(
        dao_fee_wallet: Pubkey,
        block_engine_fee_bps: u16,
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
        current_epoch: u64,
    ) -> Result<Self, TipRouterError> {
        let fee = Fees::new(dao_fee_bps, default_ncn_fee_bps, current_epoch)?;

        let mut fee_config = Self {
            block_engine_fee_bps: PodU16::from(block_engine_fee_bps),
            base_fee_wallets: [dao_fee_wallet; BaseFeeGroup::FEE_GROUP_COUNT],
            reserved: [0; 128],
            fee_1: fee,
            fee_2: fee,
        };

        fee_config.set_base_fee_wallet(BaseFeeGroup::default(), dao_fee_wallet)?;

        Ok(fee_config)
    }

    // ------------- Getters -------------
    pub fn current_fees(&self, current_epoch: u64) -> &Fees {
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

    fn updatable_fees(&mut self, current_epoch: u64) -> &mut Fees {
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

    fn update_updatable_epoch(&mut self, current_epoch: u64) -> Result<(), TipRouterError> {
        let next_epoch = current_epoch
            .checked_add(1)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let updatable_fees = self.updatable_fees(current_epoch);
        updatable_fees.set_activation_epoch(next_epoch);

        Ok(())
    }

    // ------------------- TOTALS -------------------
    pub fn total_fees_bps(&self, current_epoch: u64) -> Result<u64, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.total_fees_bps()
    }

    pub fn precise_total_fee_bps(
        &self,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.precise_total_fee_bps()
    }

    // ------------------- BLOCK ENGINE -------------------
    pub fn block_engine_fee_bps(&self) -> u16 {
        self.block_engine_fee_bps.into()
    }

    pub fn precise_block_engine_fee_bps(&self) -> Result<PreciseNumber, TipRouterError> {
        let block_engine_fee_bps = self.block_engine_fee_bps();
        PreciseNumber::new(block_engine_fee_bps.into()).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn set_block_engine_fee_bps(&mut self, value: u16) -> Result<(), TipRouterError> {
        if value as u64 > MAX_FEE_BPS {
            return Err(TipRouterError::FeeCapExceeded);
        }

        self.block_engine_fee_bps = PodU16::from(value);
        Ok(())
    }

    // ------------------- BASE -------------------

    pub fn base_fee_bps(
        &self,
        base_fee_group: BaseFeeGroup,
        current_epoch: u64,
    ) -> Result<u16, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.base_fee_bps(base_fee_group)
    }

    pub fn precise_base_fee_bps(
        &self,
        base_fee_group: BaseFeeGroup,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.precise_base_fee_bps(base_fee_group)
    }

    pub fn adjusted_base_fee_bps(
        &self,
        base_fee_group: BaseFeeGroup,
        current_epoch: u64,
    ) -> Result<u64, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        let fee = current_fees.base_fee_bps(base_fee_group)?;
        self.adjusted_fee_bps(fee)
    }

    pub fn adjusted_precise_base_fee_bps(
        &self,
        base_fee_group: BaseFeeGroup,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        let fee = current_fees.base_fee_bps(base_fee_group)?;
        self.adjusted_precise_fee_bps(fee)
    }

    pub fn set_base_fee_bps(
        &mut self,
        base_fee_group: BaseFeeGroup,
        value: u16,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        let updateable_fees = self.updatable_fees(current_epoch);
        updateable_fees.set_base_fee_bps(base_fee_group, value)
    }

    // ------------------- NCN -------------------

    pub fn ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<u16, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.ncn_fee_bps(ncn_fee_group)
    }

    pub fn precise_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        current_fees.precise_ncn_fee_bps(ncn_fee_group)
    }

    pub fn adjusted_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<u64, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        let fee = current_fees.ncn_fee_bps(ncn_fee_group)?;
        self.adjusted_fee_bps(fee)
    }

    pub fn adjusted_precise_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
        current_epoch: u64,
    ) -> Result<PreciseNumber, TipRouterError> {
        let current_fees = self.current_fees(current_epoch);
        let fee = current_fees.ncn_fee_bps(ncn_fee_group)?;
        self.adjusted_precise_fee_bps(fee)
    }

    pub fn set_ncn_fee_bps(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        value: u16,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        let updateable_fees = self.updatable_fees(current_epoch);
        updateable_fees.set_ncn_fee_bps(ncn_fee_group, value)
    }

    // ------------------- WALLETS -------------------

    pub fn base_fee_wallet(&self, base_fee_group: BaseFeeGroup) -> Result<Pubkey, TipRouterError> {
        let group_index = base_fee_group.group_index()?;
        Ok(self.base_fee_wallets[group_index])
    }

    pub fn set_base_fee_wallet(
        &mut self,
        base_fee_group: BaseFeeGroup,
        wallet: Pubkey,
    ) -> Result<(), TipRouterError> {
        let group_index = base_fee_group.group_index()?;
        self.base_fee_wallets[group_index] = wallet;
        Ok(())
    }

    // ------------- Setters -------------
    /// Updates the Fee Config
    pub fn update_fee_config(
        &mut self,
        new_block_engine_fee_bps: Option<u16>,
        base_fee_group: Option<BaseFeeGroup>,
        new_base_fee_wallet: Option<Pubkey>,
        new_base_fee_bps: Option<u16>,
        ncn_fee_group: Option<NcnFeeGroup>,
        new_ncn_fee_bps: Option<u16>,
        current_epoch: u64,
    ) -> Result<(), TipRouterError> {
        // BLOCK ENGINE
        if let Some(new_block_engine_fee_bps) = new_block_engine_fee_bps {
            self.block_engine_fee_bps = PodU16::from(new_block_engine_fee_bps);
        }

        // BASE FEE
        let base_fee_group = if let Some(base_fee_group) = base_fee_group {
            base_fee_group
        } else {
            BaseFeeGroup::default()
        };

        if let Some(new_base_fee_wallet) = new_base_fee_wallet {
            self.set_base_fee_wallet(base_fee_group, new_base_fee_wallet)?;
        }

        if let Some(new_base_fee_bps) = new_base_fee_bps {
            self.set_base_fee_bps(base_fee_group, new_base_fee_bps, current_epoch)?;
        }

        // NCN FEE
        let ncn_fee_group = if let Some(ncn_fee_group) = ncn_fee_group {
            ncn_fee_group
        } else {
            NcnFeeGroup::default()
        };

        if let Some(new_ncn_fee_bps) = new_ncn_fee_bps {
            self.set_ncn_fee_bps(ncn_fee_group, new_ncn_fee_bps, current_epoch)?;
        }

        // ACTIVATION EPOCH
        self.update_updatable_epoch(current_epoch)?;

        // CHECK FEES
        self.check_fees_okay(current_epoch)?;

        Ok(())
    }

    // ------ Helpers -----------------

    fn check_fees_okay(&self, current_epoch: u64) -> Result<(), TipRouterError> {
        for group in BaseFeeGroup::all_groups().iter() {
            let _ = self.adjusted_precise_base_fee_bps(*group, current_epoch)?;
        }

        for group in NcnFeeGroup::all_groups().iter() {
            let _ = self.adjusted_precise_ncn_fee_bps(*group, current_epoch)?;
        }

        let total_fees_bps = self.total_fees_bps(current_epoch)?;
        if total_fees_bps > MAX_FEE_BPS {
            return Err(TipRouterError::FeeCapExceeded);
        }

        Ok(())
    }

    fn adjusted_fee_bps(&self, fee: u16) -> Result<u64, TipRouterError> {
        let remaining_bps = MAX_FEE_BPS
            .checked_sub(self.block_engine_fee_bps() as u64)
            .ok_or(TipRouterError::ArithmeticOverflow)?;
        (fee as u64)
            .checked_mul(MAX_FEE_BPS)
            .and_then(|x| x.checked_div(remaining_bps))
            .ok_or(TipRouterError::DenominatorIsZero)
    }

    fn adjusted_precise_fee_bps(&self, fee: u16) -> Result<PreciseNumber, TipRouterError> {
        let remaining_bps = MAX_FEE_BPS
            .checked_sub(self.block_engine_fee_bps() as u64)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let precise_remaining_bps = PreciseNumber::new(remaining_bps as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        let adjusted_fee = (fee as u64)
            .checked_mul(MAX_FEE_BPS)
            .ok_or(TipRouterError::ArithmeticOverflow)?;

        let precise_adjusted_fee = PreciseNumber::new(adjusted_fee as u128)
            .ok_or(TipRouterError::NewPreciseNumberError)?;

        precise_adjusted_fee
            .checked_div(&precise_remaining_bps)
            .ok_or(TipRouterError::DenominatorIsZero)
    }
}

#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Fees {
    activation_epoch: PodU64,

    reserved: [u8; 128],
    base_fee_groups_bps: [Fee; 8],
    ncn_fee_groups_bps: [Fee; 8],
}

impl Fees {
    pub fn new(
        dao_fee_bps: u16,
        default_ncn_fee_bps: u16,
        epoch: u64,
    ) -> Result<Self, TipRouterError> {
        let mut fees = Self {
            activation_epoch: PodU64::from(epoch),
            reserved: [0; 128],
            base_fee_groups_bps: [Fee::default(); BaseFeeGroup::FEE_GROUP_COUNT],
            ncn_fee_groups_bps: [Fee::default(); NcnFeeGroup::FEE_GROUP_COUNT],
        };

        fees.set_base_fee_bps(BaseFeeGroup::default(), dao_fee_bps)?;
        fees.set_ncn_fee_bps(NcnFeeGroup::default(), default_ncn_fee_bps)?;

        Ok(fees)
    }

    // ------ Getters -----------------
    pub fn activation_epoch(&self) -> u64 {
        self.activation_epoch.into()
    }

    pub fn base_fee_bps(&self, base_fee_group: BaseFeeGroup) -> Result<u16, TipRouterError> {
        let group_index = base_fee_group.group_index()?;

        Ok(self.base_fee_groups_bps[group_index].fee())
    }

    pub fn precise_base_fee_bps(
        &self,
        base_fee_group: BaseFeeGroup,
    ) -> Result<PreciseNumber, TipRouterError> {
        let fee = self.base_fee_bps(base_fee_group)?;

        PreciseNumber::new(fee.into()).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn ncn_fee_bps(&self, ncn_fee_group: NcnFeeGroup) -> Result<u16, TipRouterError> {
        let group_index = ncn_fee_group.group_index()?;

        Ok(self.ncn_fee_groups_bps[group_index].fee())
    }

    pub fn precise_ncn_fee_bps(
        &self,
        ncn_fee_group: NcnFeeGroup,
    ) -> Result<PreciseNumber, TipRouterError> {
        let fee = self.ncn_fee_bps(ncn_fee_group)?;

        PreciseNumber::new(fee.into()).ok_or(TipRouterError::NewPreciseNumberError)
    }

    pub fn total_fees_bps(&self) -> Result<u64, TipRouterError> {
        let mut total_fee_bps: u64 = 0;

        for group in BaseFeeGroup::all_groups().iter() {
            let base_fee_bps = self.base_fee_bps(*group)?;

            total_fee_bps = total_fee_bps
                .checked_add(base_fee_bps as u64)
                .ok_or(TipRouterError::ArithmeticOverflow)?;
        }

        for group in NcnFeeGroup::all_groups().iter() {
            let ncn_fee_bps = self.ncn_fee_bps(*group)?;

            total_fee_bps = total_fee_bps
                .checked_add(ncn_fee_bps as u64)
                .ok_or(TipRouterError::ArithmeticOverflow)?;
        }

        Ok(total_fee_bps)
    }

    pub fn precise_total_fee_bps(&self) -> Result<PreciseNumber, TipRouterError> {
        let total_fee_bps = self.total_fees_bps()?;
        PreciseNumber::new(total_fee_bps.into()).ok_or(TipRouterError::NewPreciseNumberError)
    }

    // ------ Setters -----------------
    fn set_activation_epoch(&mut self, value: u64) {
        self.activation_epoch = PodU64::from(value);
    }

    fn set_base_fee_bps(
        &mut self,
        base_fee_group: BaseFeeGroup,
        value: u16,
    ) -> Result<(), TipRouterError> {
        if value as u64 > MAX_FEE_BPS {
            return Err(TipRouterError::FeeCapExceeded);
        }

        let group_index = base_fee_group.group_index()?;

        self.base_fee_groups_bps[group_index] = Fee::new(value);

        Ok(())
    }

    pub fn set_ncn_fee_bps(
        &mut self,
        ncn_fee_group: NcnFeeGroup,
        value: u16,
    ) -> Result<(), TipRouterError> {
        if value as u64 > MAX_FEE_BPS {
            return Err(TipRouterError::FeeCapExceeded);
        }

        let group_index = ncn_fee_group.group_index()?;

        self.ncn_fee_groups_bps[group_index] = Fee::new(value);

        Ok(())
    }
}

// ----------- FEE Because we can't do PodU16 in struct ------------
#[derive(Debug, Clone, Copy, Zeroable, ShankType, Pod)]
#[repr(C)]
pub struct Fee {
    fee: PodU16,
}

impl Default for Fee {
    fn default() -> Self {
        Self {
            fee: PodU16::from(0),
        }
    }
}

impl Fee {
    pub fn new(fee: u16) -> Self {
        Self {
            fee: PodU16::from(fee),
        }
    }

    pub fn fee(&self) -> u16 {
        self.fee.into()
    }
}

#[cfg(test)]
mod tests {
    // use solana_program::pubkey::Pubkey;

    // use super::*;

    // #[test]
    // fn test_update_fees() {
    //     const BLOCK_ENGINE_FEE: u64 = 100;
    //     const DAO_FEE: u64 = 200;
    //     const DEFAULT_NCN_FEE: u64 = 300;
    //     const STARTING_EPOCH: u64 = 10;

    //     let dao_fee_wallet = Pubkey::new_unique();

    //     let mut fee_config = FeeConfig::new(
    //         dao_fee_wallet,
    //         BLOCK_ENGINE_FEE,
    //         DAO_FEE,
    //         DEFAULT_NCN_FEE,
    //         STARTING_EPOCH,
    //     )
    //     .unwrap();

    //     assert_eq!(fee_config.fee_wallet(), dao_fee_wallet);

    //     assert_eq!(fee_config.fee_1.activation_epoch(), STARTING_EPOCH);
    //     assert_eq!(fee_config.fee_1.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
    //     assert_eq!(fee_config.fee_1.dao_fee_bps(), DAO_FEE);
    //     assert_eq!(
    //         fee_config
    //             .fee_1
    //             .ncn_fee_bps(NcnFeeGroup::default())
    //             .unwrap(),
    //         DEFAULT_NCN_FEE
    //     );

    //     assert_eq!(fee_config.fee_2.activation_epoch(), STARTING_EPOCH);
    //     assert_eq!(fee_config.fee_2.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
    //     assert_eq!(fee_config.fee_2.dao_fee_bps(), DAO_FEE);
    //     assert_eq!(
    //         fee_config
    //             .fee_2
    //             .ncn_fee_bps(NcnFeeGroup::default())
    //             .unwrap(),
    //         DEFAULT_NCN_FEE
    //     );

    //     let new_fees = Fees::new(500, 600, 700, 10).unwrap();
    //     let new_wallet = Pubkey::new_unique();

    //     fee_config
    //         .update_fee_config(
    //             Some(new_wallet),
    //             Some(new_fees.block_engine_fee_bps()),
    //             Some(new_fees.dao_fee_bps()),
    //             Some(new_fees.ncn_fee_bps(NcnFeeGroup::default()).unwrap()),
    //             None,
    //             STARTING_EPOCH,
    //         )
    //         .unwrap();

    //     assert_eq!(fee_config.fee_wallet(), new_wallet);

    //     assert_eq!(fee_config.fee_1.activation_epoch(), STARTING_EPOCH + 1);
    //     assert_eq!(fee_config.fee_1.block_engine_fee_bps(), 500);
    //     assert_eq!(fee_config.fee_1.dao_fee_bps(), 600);
    //     assert_eq!(
    //         fee_config
    //             .fee_1
    //             .ncn_fee_bps(NcnFeeGroup::default())
    //             .unwrap(),
    //         700
    //     );

    //     assert_eq!(fee_config.fee_2.activation_epoch(), STARTING_EPOCH);
    //     assert_eq!(fee_config.fee_2.block_engine_fee_bps(), 500); // This will change regardless
    //     assert_eq!(fee_config.fee_2.dao_fee_bps(), DAO_FEE);
    //     assert_eq!(
    //         fee_config
    //             .fee_2
    //             .ncn_fee_bps(NcnFeeGroup::default())
    //             .unwrap(),
    //         DEFAULT_NCN_FEE
    //     );
    // }

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

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
        if dao_fee_wallet.eq(&Pubkey::default()) {
            return Err(TipRouterError::DefaultDaoWallet);
        }

        if block_engine_fee_bps as u64 > MAX_FEE_BPS {
            return Err(TipRouterError::FeeCapExceeded);
        }

        let fee = Fees::new(dao_fee_bps, default_ncn_fee_bps, current_epoch)?;

        let mut fee_config = Self {
            block_engine_fee_bps: PodU16::from(block_engine_fee_bps),
            base_fee_wallets: [dao_fee_wallet; BaseFeeGroup::FEE_GROUP_COUNT],
            reserved: [0; 128],
            fee_1: fee,
            fee_2: fee,
        };

        fee_config.set_base_fee_wallet(BaseFeeGroup::default(), dao_fee_wallet)?;

        fee_config.check_fees_okay(current_epoch)?;

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

    fn set_fees_to_current(&mut self, current_epoch: u64) -> Result<(), TipRouterError> {
        if self.fee_1.activation_epoch() > current_epoch
            || self.fee_2.activation_epoch() > current_epoch
        {
            return Err(TipRouterError::FeeNotActive);
        }

        let cloned_current_fees = *self.current_fees(current_epoch);
        let updatable_fees = self.updatable_fees(current_epoch);
        *updatable_fees = cloned_current_fees;

        Ok(())
    }

    /// Updates the Fee Config
    #[allow(clippy::too_many_arguments)]
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
        // IF NEW CHANGES, COPY OVER CURRENT FEES
        {
            let updatable_fees = self.updatable_fees(current_epoch);
            if updatable_fees.activation_epoch() <= current_epoch {
                self.set_fees_to_current(current_epoch)?;
            }
        }

        // BLOCK ENGINE
        if let Some(new_block_engine_fee_bps) = new_block_engine_fee_bps {
            self.block_engine_fee_bps = PodU16::from(new_block_engine_fee_bps);
        }

        // BASE FEE
        let base_fee_group = base_fee_group.unwrap_or_default();

        if let Some(new_base_fee_wallet) = new_base_fee_wallet {
            self.set_base_fee_wallet(base_fee_group, new_base_fee_wallet)?;
        }

        if let Some(new_base_fee_bps) = new_base_fee_bps {
            self.set_base_fee_bps(base_fee_group, new_base_fee_bps, current_epoch)?;
        }

        // NCN FEE
        let ncn_fee_group = ncn_fee_group.unwrap_or_default();

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

    pub fn check_fees_okay(&self, current_epoch: u64) -> Result<(), TipRouterError> {
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

    pub fn set_base_fee_bps(
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
    use jito_restaking_core::MAX_FEE_BPS;
    use solana_program::pubkey::Pubkey;

    use super::*;

    #[test]
    fn test_get_all_fees() {
        const BLOCK_ENGINE_FEE: u16 = 100;
        const DAO_FEE: u16 = 200;
        const DEFAULT_NCN_FEE: u16 = 300;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();

        let fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        )
        .unwrap();

        fee_config.check_fees_okay(STARTING_EPOCH).unwrap();

        assert_eq!(fee_config.block_engine_fee_bps(), BLOCK_ENGINE_FEE);

        let dao_fee_group = BaseFeeGroup::default();

        assert_eq!(
            fee_config.base_fee_wallet(dao_fee_group).unwrap(),
            dao_fee_wallet
        );

        assert_eq!(
            fee_config.fee_1.base_fee_bps(dao_fee_group).unwrap(),
            DAO_FEE
        );
        assert_eq!(
            fee_config.fee_2.base_fee_bps(dao_fee_group).unwrap(),
            DAO_FEE
        );

        let default_ncn_fee_group = NcnFeeGroup::default();

        assert_eq!(
            fee_config.fee_1.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            DEFAULT_NCN_FEE
        );

        assert_eq!(
            fee_config.fee_2.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            DEFAULT_NCN_FEE
        );
    }

    #[test]
    fn test_init_fee_config_errors() {
        const OK_FEE: u16 = 0;
        const OK_EPOCH: u64 = 0;

        let ok_wallet = Pubkey::new_unique();

        // DEFAULT WALLET
        let error = FeeConfig::new(Pubkey::default(), OK_FEE, OK_FEE, OK_FEE, OK_EPOCH);
        assert_eq!(error.err().unwrap(), TipRouterError::DefaultDaoWallet);

        // BLOCK ENGINE FEE
        let error = FeeConfig::new(ok_wallet, MAX_FEE_BPS + 1, OK_FEE, OK_FEE, OK_EPOCH);
        assert_eq!(error.err().unwrap(), TipRouterError::FeeCapExceeded);

        // DAO FEE
        let error = FeeConfig::new(ok_wallet, OK_FEE, MAX_FEE_BPS + 1, OK_FEE, OK_EPOCH);
        assert_eq!(error.err().unwrap(), TipRouterError::FeeCapExceeded);

        // NCN FEE
        let error = FeeConfig::new(ok_wallet, OK_FEE, OK_FEE, MAX_FEE_BPS + 1, OK_EPOCH);
        assert_eq!(error.err().unwrap(), TipRouterError::FeeCapExceeded);

        // ADJUSTED FEE ERROR
        let error = FeeConfig::new(ok_wallet, MAX_FEE_BPS, OK_FEE, OK_FEE, OK_EPOCH);
        assert_eq!(error.err().unwrap(), TipRouterError::DenominatorIsZero);

        //TODO should it be an error if adjusted fee is 0?
        // let error = FeeConfig::new(ok_wallet, MAX_FEE_BPS - 1, 1000, OK_FEE, OK_EPOCH);
        // assert_eq!(error.err().unwrap(), TipRouterError::DenominatorIsZero);
    }

    #[test]
    fn test_update_fees() {
        const BLOCK_ENGINE_FEE: u16 = 100;
        const NEW_BLOCK_ENGINE_FEE: u16 = 500;
        const DAO_FEE: u16 = 200;
        const NEW_DAO_FEE: u16 = 600;
        const NEW_NEW_DAO_FEE: u16 = 800;
        const DEFAULT_NCN_FEE: u16 = 300;
        const NEW_DEFAULT_NCN_FEE: u16 = 700;
        const NEW_NEW_DEFAULT_NCN_FEE: u16 = 900;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();
        let new_dao_fee_wallet = Pubkey::new_unique();

        let mut fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        )
        .unwrap();

        fee_config
            .update_fee_config(
                Some(NEW_BLOCK_ENGINE_FEE),
                None,
                Some(new_dao_fee_wallet),
                Some(NEW_DAO_FEE),
                None,
                Some(NEW_DEFAULT_NCN_FEE),
                STARTING_EPOCH,
            )
            .unwrap();

        assert_eq!(fee_config.block_engine_fee_bps(), NEW_BLOCK_ENGINE_FEE);

        let dao_fee_group = BaseFeeGroup::default();

        assert_eq!(
            fee_config.base_fee_wallet(dao_fee_group).unwrap(),
            new_dao_fee_wallet
        );

        let current_fees = fee_config.current_fees(STARTING_EPOCH);
        let next_epoch_fees = fee_config.current_fees(STARTING_EPOCH + 1);

        assert_eq!(current_fees.base_fee_bps(dao_fee_group).unwrap(), DAO_FEE);
        assert_eq!(
            next_epoch_fees.base_fee_bps(dao_fee_group).unwrap(),
            NEW_DAO_FEE
        );

        let default_ncn_fee_group = NcnFeeGroup::default();

        assert_eq!(
            current_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            DEFAULT_NCN_FEE
        );
        assert_eq!(
            next_epoch_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            NEW_DEFAULT_NCN_FEE
        );

        // test update again
        fee_config
            .update_fee_config(
                None,
                None,
                None,
                Some(NEW_NEW_DAO_FEE),
                None,
                Some(NEW_NEW_DEFAULT_NCN_FEE),
                STARTING_EPOCH + 1,
            )
            .unwrap();

        assert_eq!(fee_config.block_engine_fee_bps(), NEW_BLOCK_ENGINE_FEE);

        let dao_fee_group = BaseFeeGroup::default();

        assert_eq!(
            fee_config.base_fee_wallet(dao_fee_group).unwrap(),
            new_dao_fee_wallet
        );

        let current_fees = fee_config.current_fees(STARTING_EPOCH + 1);
        let next_epoch_fees = fee_config.current_fees(STARTING_EPOCH + 2);

        assert_eq!(
            current_fees.base_fee_bps(dao_fee_group).unwrap(),
            NEW_DAO_FEE
        );
        assert_eq!(
            next_epoch_fees.base_fee_bps(dao_fee_group).unwrap(),
            NEW_NEW_DAO_FEE
        );

        let default_ncn_fee_group = NcnFeeGroup::default();

        assert_eq!(
            current_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            NEW_DEFAULT_NCN_FEE
        );
        assert_eq!(
            next_epoch_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            NEW_NEW_DEFAULT_NCN_FEE
        );
    }

    #[test]
    fn test_update_fees_no_change() {
        const BLOCK_ENGINE_FEE: u16 = 100;
        const DAO_FEE: u16 = 200;
        const DEFAULT_NCN_FEE: u16 = 300;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();

        let mut fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        )
        .unwrap();

        fee_config
            .update_fee_config(None, None, None, None, None, None, STARTING_EPOCH)
            .unwrap();

        assert_eq!(fee_config.block_engine_fee_bps(), BLOCK_ENGINE_FEE);

        let dao_fee_group = BaseFeeGroup::default();

        assert_eq!(
            fee_config.base_fee_wallet(dao_fee_group).unwrap(),
            dao_fee_wallet
        );

        let current_fees = fee_config.current_fees(STARTING_EPOCH);
        let next_epoch_fees = fee_config.current_fees(STARTING_EPOCH + 1);

        assert_eq!(current_fees.base_fee_bps(dao_fee_group).unwrap(), DAO_FEE);
        assert_eq!(
            next_epoch_fees.base_fee_bps(dao_fee_group).unwrap(),
            DAO_FEE
        );

        let default_ncn_fee_group = NcnFeeGroup::default();

        assert_eq!(
            current_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            DEFAULT_NCN_FEE
        );
        assert_eq!(
            next_epoch_fees.ncn_fee_bps(default_ncn_fee_group).unwrap(),
            DEFAULT_NCN_FEE
        );
    }

    #[test]
    fn test_update_different_group_fees() {
        const BLOCK_ENGINE_FEE: u16 = 100;
        const DAO_FEE: u16 = 200;
        const NEW_BASE_FEE: u16 = 500;
        const DEFAULT_NCN_FEE: u16 = 300;
        const NEW_NCN_FEE: u16 = 600;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();
        let new_base_fee = Pubkey::new_unique();

        let mut fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        )
        .unwrap();

        for base_fee_group in BaseFeeGroup::all_groups().iter() {
            fee_config
                .update_fee_config(
                    None,
                    Some(*base_fee_group),
                    Some(new_base_fee),
                    Some(NEW_BASE_FEE),
                    None,
                    None,
                    STARTING_EPOCH,
                )
                .unwrap();

            assert_eq!(
                fee_config.base_fee_wallet(*base_fee_group).unwrap(),
                new_base_fee
            );

            let current_fees = fee_config.current_fees(STARTING_EPOCH);
            let next_epoch_fees = fee_config.current_fees(STARTING_EPOCH + 1);

            if base_fee_group.group == BaseFeeGroup::default().group {
                assert_eq!(current_fees.base_fee_bps(*base_fee_group).unwrap(), DAO_FEE);
            } else {
                assert_eq!(current_fees.base_fee_bps(*base_fee_group).unwrap(), 0);
            }

            assert_eq!(
                next_epoch_fees.base_fee_bps(*base_fee_group).unwrap(),
                NEW_BASE_FEE
            );
        }

        for ncn_fee_group in NcnFeeGroup::all_groups().iter() {
            fee_config
                .update_fee_config(
                    None,
                    None,
                    None,
                    None,
                    Some(*ncn_fee_group),
                    Some(NEW_NCN_FEE),
                    STARTING_EPOCH,
                )
                .unwrap();

            let current_fees = fee_config.current_fees(STARTING_EPOCH);
            let next_epoch_fees = fee_config.current_fees(STARTING_EPOCH + 1);

            if ncn_fee_group.group == NcnFeeGroup::default().group {
                assert_eq!(
                    current_fees.ncn_fee_bps(*ncn_fee_group).unwrap(),
                    DEFAULT_NCN_FEE
                );
            } else {
                assert_eq!(current_fees.ncn_fee_bps(*ncn_fee_group).unwrap(), 0);
            }

            assert_eq!(
                next_epoch_fees.ncn_fee_bps(*ncn_fee_group).unwrap(),
                NEW_NCN_FEE
            );
        }

        assert_eq!(fee_config.block_engine_fee_bps(), BLOCK_ENGINE_FEE);
    }

    #[test]
    fn test_check_fees_okay() {
        const BLOCK_ENGINE_FEE: u16 = 100;
        const DAO_FEE: u16 = 200;
        const DEFAULT_NCN_FEE: u16 = 300;
        const STARTING_EPOCH: u64 = 10;

        let dao_fee_wallet = Pubkey::new_unique();

        let fee_config = FeeConfig::new(
            dao_fee_wallet,
            BLOCK_ENGINE_FEE,
            DAO_FEE,
            DEFAULT_NCN_FEE,
            STARTING_EPOCH,
        )
        .unwrap();

        fee_config.check_fees_okay(STARTING_EPOCH).unwrap();
    }

    #[test]
    fn test_current_fee() {
        let mut fee_config = FeeConfig::new(Pubkey::new_unique(), 100, 200, 300, 5).unwrap();

        assert_eq!(fee_config.current_fees(5).activation_epoch(), 5);

        fee_config.fee_1.set_activation_epoch(10);

        assert_eq!(fee_config.current_fees(5).activation_epoch(), 5);
        assert_eq!(fee_config.current_fees(10).activation_epoch(), 10);

        fee_config.fee_2.set_activation_epoch(15);

        assert_eq!(fee_config.current_fees(12).activation_epoch(), 10);
        assert_eq!(fee_config.current_fees(15).activation_epoch(), 15);
    }

    #[test]
    fn test_get_updatable_fee_mut() {
        let mut fee_config = FeeConfig::new(Pubkey::new_unique(), 100, 200, 300, 5).unwrap();

        let base_fee_group = BaseFeeGroup::default();

        let fees = fee_config.updatable_fees(10);
        fees.set_base_fee_bps(base_fee_group, 400).unwrap();
        fees.set_activation_epoch(11);

        assert_eq!(fee_config.fee_1.base_fee_bps(base_fee_group).unwrap(), 400);
        assert_eq!(fee_config.fee_1.activation_epoch(), 11);

        fee_config.fee_2.set_activation_epoch(13);

        let fees = fee_config.updatable_fees(12);
        fees.set_base_fee_bps(base_fee_group, 500).unwrap();
        fees.set_activation_epoch(13);

        assert_eq!(fee_config.fee_2.base_fee_bps(base_fee_group).unwrap(), 500);
        assert_eq!(fee_config.fee_2.activation_epoch(), 13);

        assert_eq!(fee_config.updatable_fees(u64::MAX).activation_epoch(), 11);
    }
}

//! A bank cache is effectively an Agave fastboot snapshot plus the account
//! storage/run directory layout needed to make repeated bank loads fast.

use {
    anyhow::{anyhow, Context, Result},
    std::{
        fs,
        path::{Path, PathBuf},
    },
};

#[derive(Debug)]
pub(crate) struct BankCachePaths {
    output_dir: PathBuf,
    bank_snapshot_cache_dir: PathBuf,
    bank_snapshot_dir: PathBuf,
    accounts_cache_dir: PathBuf,
    accounts_run_dir: PathBuf,
}

impl BankCachePaths {
    pub(crate) fn new(output_dir: PathBuf, slot: u64) -> Self {
        let bank_snapshot_cache_dir = output_dir.join("bank-snapshots");
        let accounts_cache_dir = output_dir.join("accounts");

        Self {
            output_dir,
            bank_snapshot_dir: bank_snapshot_cache_dir.join(slot.to_string()),
            accounts_run_dir: accounts_cache_dir.join("run"),
            bank_snapshot_cache_dir,
            accounts_cache_dir,
        }
    }

    pub(crate) fn bank_snapshot_cache_dir(&self) -> &Path {
        &self.bank_snapshot_cache_dir
    }

    pub(crate) fn bank_snapshot_dir(&self) -> &Path {
        &self.bank_snapshot_dir
    }

    pub(crate) fn accounts_run_dir(&self) -> &Path {
        &self.accounts_run_dir
    }

    // Target cache layout:
    // output_dir/
    //   bank-snapshots/
    //     <slot>/
    //   accounts/
    //     run/
    //     snapshot/
    //       <slot>/
    pub(crate) fn prepare_empty_dirs(&self) -> Result<()> {
        self.ensure_output_dir_absent_or_empty()?;

        fs::create_dir_all(&self.bank_snapshot_cache_dir).with_context(|| {
            format!(
                "failed to create bank snapshot cache dir {}",
                self.bank_snapshot_cache_dir.display()
            )
        })?;
        fs::create_dir_all(&self.accounts_run_dir).with_context(|| {
            format!(
                "failed to create accounts run dir {}",
                self.accounts_run_dir.display()
            )
        })?;
        fs::create_dir_all(self.accounts_cache_dir.join("snapshot")).with_context(|| {
            format!(
                "failed to create accounts snapshot dir under {}",
                self.accounts_cache_dir.display()
            )
        })?;

        Ok(())
    }

    fn ensure_output_dir_absent_or_empty(&self) -> Result<()> {
        if !self.output_dir.exists() {
            return Ok(());
        }

        if fs::read_dir(&self.output_dir)?
            .next()
            .transpose()?
            .is_none()
        {
            return Ok(());
        }

        Err(anyhow!(
            "output dir {} is not empty; refusing to mix cache state",
            self.output_dir.display()
        ))
    }

    pub(crate) fn ensure_accounts_run_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.accounts_run_dir).with_context(|| {
            format!(
                "failed to create accounts run dir {}",
                self.accounts_run_dir.display()
            )
        })
    }
}

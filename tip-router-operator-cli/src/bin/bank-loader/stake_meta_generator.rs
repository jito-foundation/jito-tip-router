use {
    anyhow::{anyhow, Result},
    borsh::de::BorshDeserialize,
    jito_priority_fee_distribution_sdk::PriorityFeeDistributionAccount,
    jito_tip_distribution_sdk::TipDistributionAccount,
    jito_tip_payment_sdk::{
        Config, CONFIG_ACCOUNT_SEED, TIP_ACCOUNT_SEED_0, TIP_ACCOUNT_SEED_1, TIP_ACCOUNT_SEED_2,
        TIP_ACCOUNT_SEED_3, TIP_ACCOUNT_SEED_4, TIP_ACCOUNT_SEED_5, TIP_ACCOUNT_SEED_6,
        TIP_ACCOUNT_SEED_7,
    },
    log::{info, warn},
    meta_merkle_tree::generated_merkle_tree::{Delegation, StakeMeta, StakeMetaCollection},
    rayon::prelude::*,
    solana_pubkey::PubkeyHasherBuilder,
    solana_runtime::{bank::Bank, stakes::StakeAccount},
    solana_sdk::{
        account::{from_account, ReadableAccount, WritableAccount},
        pubkey::Pubkey,
    },
    solana_stake_interface::{stake_history::StakeHistory, sysvar::stake_history},
    solana_vote::vote_account::VoteAccountsHashMap,
    std::{
        collections::HashMap,
        mem::size_of,
        sync::Arc,
        time::{Duration, Instant},
    },
    tip_router_operator_cli::distribution_meta::{
        DistributionMeta, TipReceiverInfo, WrappedPriorityFeeDistributionMeta,
        WrappedTipDistributionMeta,
    },
};

const MISSING_VOTER_PUBKEY_SAMPLE_LIMIT: usize = 10;
const STAKE_META_THREAD_POOL_THREADS: usize = 4;
const DELEGATIONS_BY_VOTER_PUBKEY_CAPACITY: usize = 1024;

type DelegationsByVoterPubkey = HashMap<Pubkey, Vec<Delegation>, PubkeyHasherBuilder>;

#[derive(Default, Debug)]
struct GenerateStakeMetaStats {
    total_duration: Duration,
    epoch_vote_accounts_duration: Duration,
    num_vote_accounts: usize,
    get_top_epoch_stakes_duration: Duration,
    stake_delegations_count: usize,
    group_delegations_duration: Duration,
    active_delegations_count: usize,
    inactive_delegations_count: usize,
    validators_with_delegations_count: usize,
    stake_history_loads: usize,
    stake_history_load_duration: Duration,
    stake_history_deserialize_duration: Duration,
    config_load_duration: Duration,
    tip_payment_balance_duration: Duration,
    tip_payment_pda_count: usize,
    stake_meta_loop_duration: Duration,
    validators_without_delegations_count: usize,
    generated_stake_metas_count: usize,
    total_delegated_duration: Duration,
    tip_distribution_meta_duration: Duration,
    priority_fee_distribution_meta_duration: Duration,
    delegations_sort_duration: Duration,
    tip_distribution_found_count: usize,
    tip_distribution_missing_count: usize,
    tip_distribution_wrong_owner_count: usize,
    tip_distribution_deserialize_failed_count: usize,
    tip_distribution_meta_build_failed_count: usize,
    priority_fee_distribution_found_count: usize,
    priority_fee_distribution_missing_count: usize,
    priority_fee_distribution_wrong_owner_count: usize,
    priority_fee_distribution_deserialize_failed_count: usize,
    priority_fee_distribution_meta_build_failed_count: usize,
    max_delegations_per_validator: usize,
    avg_delegations_per_validator: f64,
    final_stake_metas_sort_duration: Duration,
}

#[derive(Default)]
struct DistributionMetaStats {
    found_count: usize,
    missing_count: usize,
    wrong_owner_count: usize,
    deserialize_failed_count: usize,
    meta_build_failed_count: usize,
}

impl DistributionMetaStats {
    fn record_found(&mut self) {
        self.found_count += 1;
    }

    fn record_missing(&mut self) {
        self.missing_count += 1;
    }

    fn record_wrong_owner(&mut self) {
        self.wrong_owner_count += 1;
    }

    fn record_deserialize_failed(&mut self) {
        self.deserialize_failed_count += 1;
    }

    fn record_meta_build_failed(&mut self) {
        self.meta_build_failed_count += 1;
    }

    fn merge(&mut self, other: Self) {
        self.found_count += other.found_count;
        self.missing_count += other.missing_count;
        self.wrong_owner_count += other.wrong_owner_count;
        self.deserialize_failed_count += other.deserialize_failed_count;
        self.meta_build_failed_count += other.meta_build_failed_count;
    }
}

#[derive(Default)]
struct StakeMetaBuildStats {
    total_delegations: usize,
    total_delegated_duration: Duration,
    tip_distribution_meta_duration: Duration,
    priority_fee_distribution_meta_duration: Duration,
    delegations_sort_duration: Duration,
    tip_distribution: DistributionMetaStats,
    priority_fee_distribution: DistributionMetaStats,
    max_delegations_per_validator: usize,
}

impl StakeMetaBuildStats {
    fn merge(&mut self, other: Self) {
        self.total_delegations += other.total_delegations;
        self.total_delegated_duration += other.total_delegated_duration;
        self.tip_distribution_meta_duration += other.tip_distribution_meta_duration;
        self.priority_fee_distribution_meta_duration +=
            other.priority_fee_distribution_meta_duration;
        self.delegations_sort_duration += other.delegations_sort_duration;
        self.tip_distribution.merge(other.tip_distribution);
        self.priority_fee_distribution
            .merge(other.priority_fee_distribution);
        self.max_delegations_per_validator = self
            .max_delegations_per_validator
            .max(other.max_delegations_per_validator);
    }

    fn apply_to(self, stats: &mut GenerateStakeMetaStats) {
        stats.total_delegated_duration += self.total_delegated_duration;
        stats.tip_distribution_meta_duration += self.tip_distribution_meta_duration;
        stats.priority_fee_distribution_meta_duration +=
            self.priority_fee_distribution_meta_duration;
        stats.delegations_sort_duration += self.delegations_sort_duration;
        stats.tip_distribution_found_count += self.tip_distribution.found_count;
        stats.tip_distribution_missing_count += self.tip_distribution.missing_count;
        stats.tip_distribution_wrong_owner_count += self.tip_distribution.wrong_owner_count;
        stats.tip_distribution_deserialize_failed_count +=
            self.tip_distribution.deserialize_failed_count;
        stats.tip_distribution_meta_build_failed_count +=
            self.tip_distribution.meta_build_failed_count;
        stats.priority_fee_distribution_found_count += self.priority_fee_distribution.found_count;
        stats.priority_fee_distribution_missing_count +=
            self.priority_fee_distribution.missing_count;
        stats.priority_fee_distribution_wrong_owner_count +=
            self.priority_fee_distribution.wrong_owner_count;
        stats.priority_fee_distribution_deserialize_failed_count +=
            self.priority_fee_distribution.deserialize_failed_count;
        stats.priority_fee_distribution_meta_build_failed_count +=
            self.priority_fee_distribution.meta_build_failed_count;
        stats.max_delegations_per_validator = stats
            .max_delegations_per_validator
            .max(self.max_delegations_per_validator);
    }
}

struct StakeMetaBuildInput {
    vote_pubkey: Pubkey,
    validator_node_pubkey: Pubkey,
    commission: u8,
    delegations: Vec<Delegation>,
}

struct StakeMetaBuildOutput {
    stake_meta: StakeMeta,
    stats: StakeMetaBuildStats,
}

#[derive(Default)]
struct StakeMetaBuildBatch {
    stake_metas: Vec<StakeMeta>,
    stats: StakeMetaBuildStats,
}

impl StakeMetaBuildBatch {
    fn push(&mut self, output: StakeMetaBuildOutput) {
        self.stake_metas.push(output.stake_meta);
        self.stats.merge(output.stats);
    }

    fn merge(mut self, mut other: Self) -> Self {
        self.stake_metas.append(&mut other.stake_metas);
        self.stats.merge(other.stats);
        self
    }
}

impl GenerateStakeMetaStats {
    fn log(&self) {
        info!(
            "stake_meta_stats total_duration_ms={} epoch_vote_accounts_duration_ms={} num_vote_accounts={} get_top_epoch_stakes_duration_ms={} stake_delegations_count={} group_delegations_duration_ms={} active_delegations_count={} inactive_delegations_count={} validators_with_delegations_count={} validators_without_delegations_count={} generated_stake_metas_count={} final_stake_metas_sort_duration_ms={}",
            self.total_duration.as_millis(),
            self.epoch_vote_accounts_duration.as_millis(),
            self.num_vote_accounts,
            self.get_top_epoch_stakes_duration.as_millis(),
            self.stake_delegations_count,
            self.group_delegations_duration.as_millis(),
            self.active_delegations_count,
            self.inactive_delegations_count,
            self.validators_with_delegations_count,
            self.validators_without_delegations_count,
            self.generated_stake_metas_count,
            self.final_stake_metas_sort_duration.as_millis(),
        );
        info!(
            "stake_meta_loop_stats stake_history_loads={} stake_history_load_duration_ms={} stake_history_deserialize_duration_ms={} config_load_duration_ms={} tip_payment_balance_duration_ms={} tip_payment_pda_count={} stake_meta_loop_duration_ms={} total_delegated_duration_ms={} tip_distribution_meta_duration_ms={} priority_fee_distribution_meta_duration_ms={} delegations_sort_duration_ms={} max_delegations_per_validator={} avg_delegations_per_validator={:.2}",
            self.stake_history_loads,
            self.stake_history_load_duration.as_millis(),
            self.stake_history_deserialize_duration.as_millis(),
            self.config_load_duration.as_millis(),
            self.tip_payment_balance_duration.as_millis(),
            self.tip_payment_pda_count,
            self.stake_meta_loop_duration.as_millis(),
            self.total_delegated_duration.as_millis(),
            self.tip_distribution_meta_duration.as_millis(),
            self.priority_fee_distribution_meta_duration.as_millis(),
            self.delegations_sort_duration.as_millis(),
            self.max_delegations_per_validator,
            self.avg_delegations_per_validator,
        );
        info!(
            "stake_meta_distribution_stats tip_found={} tip_missing={} tip_wrong_owner={} tip_deserialize_failed={} tip_meta_build_failed={} priority_fee_found={} priority_fee_missing={} priority_fee_wrong_owner={} priority_fee_deserialize_failed={} priority_fee_meta_build_failed={}",
            self.tip_distribution_found_count,
            self.tip_distribution_missing_count,
            self.tip_distribution_wrong_owner_count,
            self.tip_distribution_deserialize_failed_count,
            self.tip_distribution_meta_build_failed_count,
            self.priority_fee_distribution_found_count,
            self.priority_fee_distribution_missing_count,
            self.priority_fee_distribution_wrong_owner_count,
            self.priority_fee_distribution_deserialize_failed_count,
            self.priority_fee_distribution_meta_build_failed_count,
        );
    }
}

pub(crate) fn generate_stake_meta_collection_with_stats(
    bank: &Arc<Bank>,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_payment_program_id: &Pubkey,
) -> Result<StakeMetaCollection> {
    assert!(bank.is_frozen());

    let total_started = Instant::now();
    let mut stats = GenerateStakeMetaStats::default();

    let phase_started = Instant::now();
    let epoch_vote_accounts = bank.epoch_vote_accounts(bank.epoch()).ok_or_else(|| {
        anyhow!(
            "no vote accounts for slot {} epoch {}",
            bank.slot(),
            bank.epoch()
        )
    })?;
    stats.epoch_vote_accounts_duration = phase_started.elapsed();
    stats.num_vote_accounts = epoch_vote_accounts.len();

    let phase_started = Instant::now();
    let top_epoch_stakes = bank.get_top_epoch_stakes();
    stats.get_top_epoch_stakes_duration = phase_started.elapsed();
    let delegations = top_epoch_stakes.stake_delegations();
    stats.stake_delegations_count = delegations.len();

    let phase_started = Instant::now();
    let mut voter_pubkey_to_delegations =
        group_delegations_by_voter_pubkey_with_stats(delegations, bank, &mut stats);
    stats.group_delegations_duration = phase_started.elapsed();
    stats.validators_with_delegations_count = voter_pubkey_to_delegations.len();

    let phase_started = Instant::now();
    let (config_pda, _) =
        Pubkey::find_program_address(&[CONFIG_ACCOUNT_SEED], tip_payment_program_id);
    let config = get_config(bank, &config_pda)?;
    stats.config_load_duration = phase_started.elapsed();

    let bb_commission_pct = config.block_builder_commission_pct;
    let tip_receiver = config.tip_receiver;

    let phase_started = Instant::now();
    let tip_pdas = derive_tip_payment_pubkeys(tip_payment_program_id);
    stats.tip_payment_pda_count = tip_pdas.len();

    let excess_tip_balances: u64 = tip_pdas
        .iter()
        .map(|pubkey| {
            let tip_account = bank.get_account(pubkey).expect("tip account exists");
            tip_account
                .lamports()
                .checked_sub(bank.get_minimum_balance_for_rent_exemption(tip_account.data().len()))
                .expect("tip balance underflow")
        })
        .sum();
    let block_builder_tips = excess_tip_balances
        .checked_mul(bb_commission_pct)
        .expect("block_builder_tips overflow")
        .checked_div(100)
        .expect("block_builder_tips division error");
    let tip_receiver_fee = excess_tip_balances
        .checked_sub(block_builder_tips)
        .expect("tip_receiver_fee doesnt underflow");
    stats.tip_payment_balance_duration = phase_started.elapsed();

    let phase_started = Instant::now();
    let (stake_meta_build_inputs, missing_voter_pubkey_count, missing_voter_pubkey_sample) =
        take_stake_meta_build_inputs(epoch_vote_accounts, &mut voter_pubkey_to_delegations);
    stats.validators_without_delegations_count = missing_voter_pubkey_count;

    let StakeMetaBuildBatch {
        mut stake_metas,
        stats: build_stats,
    } = build_stake_metas_with_thread_pool(
        bank,
        tip_distribution_program_id,
        priority_fee_distribution_program_id,
        tip_receiver,
        tip_receiver_fee,
        stake_meta_build_inputs,
    )?;
    stats.stake_meta_loop_duration = phase_started.elapsed();

    warn_missing_voter_pubkeys(
        stats.validators_without_delegations_count,
        &missing_voter_pubkey_sample,
    );
    stats.generated_stake_metas_count = stake_metas.len();
    if stats.generated_stake_metas_count > 0 {
        stats.avg_delegations_per_validator =
            build_stats.total_delegations as f64 / stats.generated_stake_metas_count as f64;
    }
    build_stats.apply_to(&mut stats);

    let phase_started = Instant::now();
    stake_metas.sort();
    stats.final_stake_metas_sort_duration = phase_started.elapsed();

    let stake_meta_collection = StakeMetaCollection {
        stake_metas,
        tip_distribution_program_id: tip_distribution_program_id.to_owned(),
        priority_fee_distribution_program_id: priority_fee_distribution_program_id.to_owned(),
        bank_hash: bank.hash().to_string(),
        epoch: bank.epoch(),
        slot: bank.slot(),
    };
    stats.total_duration = total_started.elapsed();
    stats.log();

    Ok(stake_meta_collection)
}

fn take_stake_meta_build_inputs(
    epoch_vote_accounts: &VoteAccountsHashMap,
    voter_pubkey_to_delegations: &mut DelegationsByVoterPubkey,
) -> (Vec<StakeMetaBuildInput>, usize, Vec<Pubkey>) {
    let mut stake_meta_build_inputs = Vec::with_capacity(epoch_vote_accounts.len());
    let mut missing_voter_pubkey_count = 0usize;
    let mut missing_voter_pubkey_sample = Vec::new();

    for (vote_pubkey, (_, vote_account)) in epoch_vote_accounts.iter() {
        let Some(delegations) = voter_pubkey_to_delegations.remove(vote_pubkey) else {
            missing_voter_pubkey_count += 1;
            record_missing_voter_pubkey_sample(&mut missing_voter_pubkey_sample, vote_pubkey);
            continue;
        };

        let vote_state = vote_account.vote_state_view();
        stake_meta_build_inputs.push(StakeMetaBuildInput {
            vote_pubkey: *vote_pubkey,
            validator_node_pubkey: *vote_state.node_pubkey(),
            commission: vote_state.commission(),
            delegations,
        });
    }

    (
        stake_meta_build_inputs,
        missing_voter_pubkey_count,
        missing_voter_pubkey_sample,
    )
}

fn build_stake_metas_with_thread_pool(
    bank: &Arc<Bank>,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_receiver: Pubkey,
    tip_receiver_fee: u64,
    stake_meta_build_inputs: Vec<StakeMetaBuildInput>,
) -> Result<StakeMetaBuildBatch> {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(STAKE_META_THREAD_POOL_THREADS)
        .thread_name(|index| format!("stake-meta-{index}"))
        .build()?;

    Ok(thread_pool.install(|| {
        stake_meta_build_inputs
            .into_par_iter()
            .map(|stake_meta_build_input| {
                build_stake_meta(
                    bank,
                    tip_distribution_program_id,
                    priority_fee_distribution_program_id,
                    tip_receiver,
                    tip_receiver_fee,
                    stake_meta_build_input,
                )
            })
            .fold(StakeMetaBuildBatch::default, |mut batch, output| {
                batch.push(output);
                batch
            })
            .reduce(StakeMetaBuildBatch::default, StakeMetaBuildBatch::merge)
    }))
}

fn build_stake_meta(
    bank: &Arc<Bank>,
    tip_distribution_program_id: &Pubkey,
    priority_fee_distribution_program_id: &Pubkey,
    tip_receiver: Pubkey,
    tip_receiver_fee: u64,
    stake_meta_build_input: StakeMetaBuildInput,
) -> StakeMetaBuildOutput {
    let StakeMetaBuildInput {
        vote_pubkey,
        validator_node_pubkey,
        commission,
        mut delegations,
    } = stake_meta_build_input;

    let mut stats = StakeMetaBuildStats {
        total_delegations: delegations.len(),
        max_delegations_per_validator: delegations.len(),
        ..StakeMetaBuildStats::default()
    };

    let subphase_started = Instant::now();
    let total_delegated = delegations.iter().fold(0u64, |sum, delegation| {
        sum.checked_add(delegation.lamports_delegated)
            .expect("total delegated lamports should not overflow u64")
    });
    stats.total_delegated_duration += subphase_started.elapsed();

    let subphase_started = Instant::now();
    let (maybe_tip_distribution_meta, tip_distribution_stats) =
        get_distribution_meta_with_stats::<TipDistributionAccount, WrappedTipDistributionMeta>(
            bank,
            tip_distribution_program_id,
            &vote_pubkey,
            Some(TipReceiverInfo {
                tip_receiver,
                tip_receiver_fee,
            }),
        );
    stats.tip_distribution_meta_duration += subphase_started.elapsed();
    stats.tip_distribution.merge(tip_distribution_stats);

    let subphase_started = Instant::now();
    let (maybe_priority_fee_distribution_meta, priority_fee_distribution_stats) =
        get_distribution_meta_with_stats::<
            PriorityFeeDistributionAccount,
            WrappedPriorityFeeDistributionMeta,
        >(
            bank,
            priority_fee_distribution_program_id,
            &vote_pubkey,
            None,
        );
    stats.priority_fee_distribution_meta_duration += subphase_started.elapsed();
    stats
        .priority_fee_distribution
        .merge(priority_fee_distribution_stats);

    let subphase_started = Instant::now();
    delegations.sort_unstable_by(|a, b| a.stake_account_pubkey.cmp(&b.stake_account_pubkey));
    stats.delegations_sort_duration += subphase_started.elapsed();

    StakeMetaBuildOutput {
        stake_meta: StakeMeta {
            maybe_tip_distribution_meta: maybe_tip_distribution_meta.map(|x| x.0),
            maybe_priority_fee_distribution_meta: maybe_priority_fee_distribution_meta.map(|x| x.0),
            validator_node_pubkey,
            validator_vote_account: vote_pubkey,
            delegations,
            total_delegated,
            commission,
        },
        stats,
    }
}

fn get_distribution_meta_with_stats<DistributionAccount, DistMeta>(
    bank: &Arc<Bank>,
    program_id: &Pubkey,
    vote_pubkey: &Pubkey,
    tip_receiver_info: Option<TipReceiverInfo>,
) -> (Option<DistMeta>, DistributionMetaStats)
where
    DistributionAccount: BorshDeserialize,
    DistMeta: DistributionMeta<DistributionAccountType = DistributionAccount>,
{
    let mut stats = DistributionMetaStats::default();
    let distribution_account_pubkey =
        DistMeta::derive_distribution_account_address(program_id, vote_pubkey, bank.epoch());
    let Some(mut account_data) = bank.get_account(&distribution_account_pubkey) else {
        stats.record_missing();
        return (None, stats);
    };

    if account_data.owner() != program_id {
        stats.record_wrong_owner();
        return (None, stats);
    }

    let Some(distribution_account_data) = account_data.data().get(8..) else {
        stats.record_deserialize_failed();
        return (None, stats);
    };
    let distribution_account =
        DistributionAccount::deserialize(&mut &distribution_account_data[..]).ok();
    let Some(distribution_account) = distribution_account else {
        stats.record_deserialize_failed();
        return (None, stats);
    };

    if let Some(tip_receiver_info) = tip_receiver_info {
        if distribution_account_pubkey == tip_receiver_info.tip_receiver {
            account_data.set_lamports(
                account_data
                    .lamports()
                    .checked_add(tip_receiver_info.tip_receiver_fee)
                    .expect("tip overflow"),
            );
        }
    }

    let actual_len = account_data.data().len();
    let expected_len = 8_usize.saturating_add(size_of::<DistributionAccount>());
    if actual_len != expected_len {
        warn!("len mismatch actual={actual_len}, expected={expected_len}");
    }
    let rent_exempt_amount = bank.get_minimum_balance_for_rent_exemption(account_data.data().len());

    let distribution_meta = DistMeta::new_from_account(
        distribution_account,
        account_data,
        distribution_account_pubkey,
        rent_exempt_amount,
    )
    .ok();
    if distribution_meta.is_some() {
        stats.record_found();
    } else {
        stats.record_meta_build_failed();
    }

    (distribution_meta, stats)
}

fn get_config(bank: &Arc<Bank>, config_pubkey: &Pubkey) -> Result<Config> {
    bank.get_account(config_pubkey)
        .ok_or_else(|| anyhow!("config account not found in bank"))
        .and_then(|config_account| {
            Config::deserialize(config_account.data())
                .map_err(|_| anyhow!("failed to deserialize config"))
        })
}

fn derive_tip_payment_pubkeys(program_id: &Pubkey) -> [Pubkey; 8] {
    [
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_0], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_1], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_2], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_3], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_4], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_5], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_6], program_id).0,
        Pubkey::find_program_address(&[TIP_ACCOUNT_SEED_7], program_id).0,
    ]
}

fn group_delegations_by_voter_pubkey_with_stats(
    delegations: &im::HashMap<Pubkey, StakeAccount>,
    bank: &Bank,
    stats: &mut GenerateStakeMetaStats,
) -> DelegationsByVoterPubkey {
    let mut delegations_by_voter_pubkey = DelegationsByVoterPubkey::with_capacity_and_hasher(
        DELEGATIONS_BY_VOTER_PUBKEY_CAPACITY,
        PubkeyHasherBuilder::default(),
    );
    let epoch = bank.epoch();
    let new_rate_activation_epoch = bank.new_warmup_cooldown_rate_epoch();

    stats.stake_history_loads += 1;
    let phase_started = Instant::now();
    let stake_history_account = bank
        .get_account(&stake_history::id())
        .expect("stake history sysvar account should be present in the loaded bank");
    stats.stake_history_load_duration += phase_started.elapsed();

    let phase_started = Instant::now();
    let stake_history = from_account::<StakeHistory, _>(&stake_history_account)
        .expect("stake history sysvar account should deserialize");
    stats.stake_history_deserialize_duration += phase_started.elapsed();

    for (stake_pubkey, stake_account) in delegations {
        let delegation = stake_account.delegation();
        let active_stake = delegation.stake(epoch, &stake_history, new_rate_activation_epoch);
        if active_stake == 0 {
            stats.inactive_delegations_count += 1;
            continue;
        }

        stats.active_delegations_count += 1;
        let authorized = stake_account.stake_state().authorized().unwrap_or_default();
        delegations_by_voter_pubkey
            .entry(delegation.voter_pubkey)
            .or_default()
            .push(Delegation {
                stake_account_pubkey: *stake_pubkey,
                staker_pubkey: authorized.staker,
                withdrawer_pubkey: authorized.withdrawer,
                lamports_delegated: delegation.stake,
            });
    }

    delegations_by_voter_pubkey
}

fn record_missing_voter_pubkey_sample(sample: &mut Vec<Pubkey>, vote_pubkey: &Pubkey) {
    if sample.len() < MISSING_VOTER_PUBKEY_SAMPLE_LIMIT {
        sample.push(*vote_pubkey);
    }
}

fn warn_missing_voter_pubkeys(missing_count: usize, sample: &[Pubkey]) {
    if missing_count == 0 {
        return;
    }

    let sample = sample
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    warn!(
        "voter_pubkeys not found in voter_pubkey_to_delegations map count={} sample_validator_vote_pubkeys=[{}]",
        missing_count, sample
    );
}

# Merkle Root Upload Monitoring Plan

## Current State

The tip router system currently tracks merkle root uploads via the `tip_router_cli.set_merkle_root` datapoint with these fields:
- `num_success`: Number of merkle roots successfully set
- `num_failed`: Number of merkle roots that failed to set
- `epoch`: The epoch being processed
- `operator_address`: Which operator is setting the roots

## Problem

We don't currently track the **total expected** number of merkle roots that need to be set for an epoch. This makes it difficult to monitor whether all merkle roots have been completely uploaded.

## Architecture Context

- **One merkle root per validator per epoch** (for tip distribution accounts)
- **Additional merkle roots for priority fee distribution accounts**
- Total expected = `tip_distribution_accounts.len() + priority_fee_distribution_accounts.len()`
- This data is already available in the code at upload time

## Proposed Solutions

### Option 1: Add Total Expected Count (Recommended)

**Code Changes:**
Modify `tip-router-operator-cli/src/submit.rs` around line 265:

```rust
datapoint_info!(
    "tip_router_cli.set_merkle_root",
    ("operator_address", operator_address.to_string(), String),
    ("epoch", tip_router_target_epoch, i64),
    ("num_success", num_success, i64),
    ("num_failed", num_failed, i64),
    ("num_expected", tip_distribution_accounts.len() + priority_fee_distribution_accounts.len(), i64),
    "cluster" => cluster,
);
```

**Grafana Alert Logic:**
- **Query A**: `SELECT last("num_success") + last("num_failed") FROM "tip_router_cli.set_merkle_root" WHERE time >= now() - 1h GROUP BY "epoch"`
- **Query B**: `SELECT last("num_expected") FROM "tip_router_cli.set_merkle_root" WHERE time >= now() - 1h GROUP BY "epoch"`
- **Alert Condition**: `$A < $B` (incomplete uploads)
- **Alert Timing**: Fire after 30 minutes to allow processing time

**Benefits:**
- Minimal code change
- Immediate alerting capability
- Uses existing datapoint structure

### Option 2: Separate Summary Datapoint

**Code Changes:**
Add a new summary datapoint after merkle root processing completes:

```rust
datapoint_info!(
    "tip_router_cli.set_merkle_root_summary",
    ("operator_address", operator_address.to_string(), String),
    ("epoch", tip_router_target_epoch, i64),
    ("total_validators", total_validators, i64),
    ("completed_uploads", completed_uploads, i64),
    ("completion_percentage", completion_percentage, f64),
    ("is_complete", is_complete, bool),
    "cluster" => cluster,
);
```

**Benefits:**
- Clear separation of concerns
- Rich completion metadata
- Boolean completion flag for simple alerting

### Option 3: Query-Based Monitoring

**Approach:**
Use existing data with more sophisticated queries:
- Track progress accumulation over time
- Monitor for stalled progress (similar to claims monitoring)
- Compare against external validator count metrics

**Benefits:**
- No code changes required
- Leverages existing data

**Drawbacks:**
- Complex query logic
- Requires external validator count data
- Less reliable than direct tracking

## Recommended Implementation

**Option 1** is the recommended approach because:
1. **Minimal code impact** - Single line addition
2. **Immediate value** - Enables direct completion monitoring
3. **Simple alerting** - Straightforward Grafana queries
4. **Reliable data** - Uses exact counts from the upload process

## Implementation Steps

1. **Update datapoint** in `submit.rs` to include `num_expected`
2. **Test locally** to verify metric collection
3. **Create Grafana alert** with completion logic
4. **Monitor and tune** alert timing and thresholds

## Related Monitoring

This complements existing monitoring:
- **Claims progress monitoring** (already implemented)
- **Voting completion monitoring** (already implemented)
- **Upload progress tracking** (via epoch state metrics)

## Alert Examples

### Incomplete Merkle Root Uploads
- **Condition**: `num_success + num_failed < num_expected`
- **Duration**: 30 minutes
- **Severity**: Warning
- **Message**: "Merkle root uploads incomplete for epoch {epoch}. Expected: {num_expected}, Completed: {num_success + num_failed}"

### Failed Merkle Root Uploads
- **Condition**: `num_failed > 0`
- **Duration**: 5 minutes
- **Severity**: Critical
- **Message**: "Merkle root upload failures for epoch {epoch}. Failed: {num_failed}, Success: {num_success}"
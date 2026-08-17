# Snapshot daemon

`snapshot-daemon` watches finalized Solana state and creates a full snapshot
after each epoch rollover. Once the new epoch is finalized, it calculates the
previous epoch's theoretical last slot and searches backward through at most 16
slots for the latest finalized slot containing a block. This keeps polling
latency from moving the snapshot target into the new epoch and handles skipped
slots at the boundary.

The daemon chooses only source archives that can reconstruct the requested
target: the highest full snapshot at or before the target, plus the highest
incremental snapshot at or before the target whose base matches that full
snapshot. It exposes only those archives to `agave-ledger-tool` through
temporary symlink directories and forces archive loading with
`--use-snapshot-archives-at-startup always`.

## Build and install

Build the release binary from the workspace root, then install it and the
included unit file:

```sh
cargo build --release -p snapshot-daemon
sudo install -m 0755 target/release/snapshot-daemon /usr/local/bin/snapshot-daemon
sudo install -m 0644 snapshot-daemon/snapshot-daemon.service /etc/systemd/system/snapshot-daemon.service
```

The unit runs as the `solana` user by default. Change `User` and `Group` in the
unit if the validator uses another account. That account must be able to read
the ledger and source snapshot archives, write the output directory, and
execute `agave-ledger-tool`.

Create the output directory and give the service account access to it:

```sh
sudo install -d -o solana -g solana -m 0750 /mnt/solana/snapshots
```

## Configure systemd

Site-specific settings belong in `/etc/default/snapshot-daemon`. The file is
optional, but the checked-in paths are examples and normally need overriding:

```ini
SNAPSHOT_DAEMON_RPC_URL=http://127.0.0.1:8899
SNAPSHOT_DAEMON_LEDGER_PATH=/path/to/validator/ledger
SNAPSHOT_DAEMON_OUTPUT_DIR=/path/to/generated/snapshots
SNAPSHOT_DAEMON_LEDGER_TOOL=/absolute/path/to/agave-ledger-tool
RUST_LOG=info
```

By default, full and incremental source snapshot archives are read from the
ledger path. If they live elsewhere, add the corresponding CLI options through
the extra-arguments setting:

```ini
SNAPSHOT_DAEMON_EXTRA_ARGS=--full-snapshot-archive-path /path/to/full --incremental-snapshot-archive-path /path/to/incremental
```

The unit deliberately sets these resource limits in its `[Service]` section:

```ini
LimitMEMLOCK=2000000000
LimitNOFILE=1000000
```

They are systemd directives, not shell environment variables. The larger
locked-memory limit is required to avoid `Cannot allocate memory (os error 12)`
during ledger-tool Bank serialization and to allow io_uring initialization.
The daemon also invokes ledger-tool with `--ignore-ulimit-nofile-error`, so a
nonfatal attempt by ledger-tool to raise its own nofile limit does not abort the
snapshot.

Reload systemd, start the daemon, and follow its logs:

```sh
sudo systemctl daemon-reload
sudo systemctl enable --now snapshot-daemon.service
sudo systemctl status snapshot-daemon.service
sudo journalctl -u snapshot-daemon.service -f
```

To apply later unit or environment-file changes:

```sh
sudo systemctl daemon-reload
sudo systemctl restart snapshot-daemon.service
```

## Startup test snapshot

To test snapshot creation without waiting for an epoch rollover, stop the
normal service and set:

```ini
SNAPSHOT_DAEMON_EXTRA_ARGS=--test-slot-ahead 100
```

Then start the service and watch the journal. At startup, the daemon records
the current finalized slot, waits until finalization has advanced by at least
100 slot numbers, and uses the first finalized slot it actually observes at or
beyond that target. Remove the extra argument and restart the service after the
test so it resumes normal epoch monitoring.

A successful test produced target slot `439470211` and archive:

```text
snapshot-439470211-ECzirB2mVVFxzQchbqdFgmow1M5jZcBHBLFUwcFUMT1b.tar.zst
```

The reconstructed Bank is expected to have slot `439470211` and bank hash
`CNH1EioUFEXavePMNQtWYmT8iEhJMhWTPLj5Nc3XHf2y`.

## Historical backfill

Set `SNAPSHOT_DAEMON_EXTRA_ARGS=--start-slot SLOT` to create snapshots for
completed epoch boundaries after `SLOT` before entering normal monitoring.
`--start-slot` and `--test-slot-ahead` conflict and cannot be used together.

Historical backfill uses the same boundary rule as normal monitoring: for each
completed epoch, it searches backward from the theoretical last slot for the
latest finalized slot containing a block. `--test-slot-ahead` remains different:
it uses the first finalized slot observed at or beyond its test target. Snapshot
compression remains the default single-stream zstd implementation.

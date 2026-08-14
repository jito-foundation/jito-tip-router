# Snapshot Daemon

A small Python daemon (3.12 or newer) that waits for finalized Solana epoch transitions and creates a full snapshot at the last populated slot of the completed epoch.

RPC access uses the asynchronous client from [`solana-py`](https://michaelhly.com/solana-py/).

## Run

```shell
uv sync
uv run main.py \
  --rpc-url http://127.0.0.1:8899 \
  --ledger-tool-bin /path/to/agave-ledger-tool \
  --ledger /path/to/ledger \
  --output-dir /path/to/snapshot-output
```

On a host without `uv`, install the pinned dependencies into a virtualenv with `pip` instead. Every dependency ships a binary wheel, so no toolchain is required:

```shell
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
.venv/bin/python main.py \
  --rpc-url http://127.0.0.1:8899 \
  --ledger-tool-bin /path/to/agave-ledger-tool \
  --ledger /path/to/ledger \
  --output-dir /path/to/snapshot-output
```

Regenerate `requirements.txt` from the lock whenever dependencies change:

```shell
uv export --no-emit-project --no-dev --format requirements-txt -o requirements.txt
```

The daemon remembers the finalized epoch observed at startup, so it always waits for a new epoch boundary instead of processing an earlier one. At the transition, it checks the theoretical final slot and up to 15 preceding slots. The first slot for which finalized `getBlock` succeeds is passed to:

```text
agave-ledger-tool --ledger <LEDGER> create-snapshot <SLOT> <OUTPUT_DIR>
```

Both stdout and stderr from the ledger tool are streamed through the daemon's logger at `INFO`. A nonzero exit status is logged at `ERROR`, and the daemon moves on to the next epoch.

## Assumptions

- The validator is local, running, and caught up.
- The RPC supports finalized `getEpochInfo` and `getBlock` requests.
- The ledger contains the history needed to replay through the selected slot.
- The ledger contains usable source snapshot archives for `agave-ledger-tool` startup.

The ledger tool's snapshot-archive startup mode opens the blockstore read-only, allowing it to run alongside the validator. This extra RocksDB access can temporarily degrade validator disk and database performance.

## Prototype scope

This first version deliberately omits persistent state, catch-up snapshots, duplicate archive detection, adaptive retries, custom signal forwarding, snapshot retries, and deployment configuration. See [PROTOTYPE_PLAN.md](PROTOTYPE_PLAN.md) for the exact prototype boundary and [SPECIFICATION_DIALOGUE.md](SPECIFICATION_DIALOGUE.md) for the broader design discussion.

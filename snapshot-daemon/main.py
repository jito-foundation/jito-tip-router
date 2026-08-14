from __future__ import annotations

import argparse
import asyncio
import logging
from dataclasses import dataclass
from pathlib import Path

from agave_snapshot import AgaveLedgerSnapshotCreator
from snapshot_daemon import EpochBoundarySnapshotDaemon
from solana_rpc import SolanaRpcClient


LOGGER = logging.getLogger("snapshot-daemon")


@dataclass(frozen=True, slots=True)
class SnapshotDaemonConfig:
    rpc_url: str
    ledger_tool_bin: Path
    ledger_path: Path
    output_dir: Path


def parse_snapshot_daemon_config() -> SnapshotDaemonConfig:
    parser = argparse.ArgumentParser(
        description="Create a full snapshot at the last populated slot of each Solana epoch."
    )
    parser.add_argument("--rpc-url", required=True, help="Solana JSON-RPC URL")
    parser.add_argument(
        "--ledger-tool-bin",
        required=True,
        type=Path,
        help="path to the agave-ledger-tool executable",
    )
    parser.add_argument(
        "--ledger",
        required=True,
        type=Path,
        dest="ledger_path",
        help="path to the local validator ledger",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        type=Path,
        help="directory where full snapshot archives will be written",
    )
    arguments = parser.parse_args()
    return SnapshotDaemonConfig(
        rpc_url=arguments.rpc_url,
        ledger_tool_bin=arguments.ledger_tool_bin,
        ledger_path=arguments.ledger_path,
        output_dir=arguments.output_dir,
    )


def configure_console_logging() -> None:
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(name)s: %(message)s",
    )


async def run_snapshot_daemon(config: SnapshotDaemonConfig) -> None:
    rpc_client = SolanaRpcClient(config.rpc_url)
    snapshot_creator = AgaveLedgerSnapshotCreator(
        config.ledger_tool_bin,
        config.ledger_path,
        config.output_dir,
    )
    snapshot_daemon = EpochBoundarySnapshotDaemon(rpc_client, snapshot_creator)

    try:
        await snapshot_daemon.capture_snapshots_at_epoch_boundaries()
    finally:
        await rpc_client.close()


def main() -> int:
    configure_console_logging()
    config = parse_snapshot_daemon_config()

    try:
        config.output_dir.mkdir(parents=True, exist_ok=True)
    except OSError as error:
        LOGGER.error("could not create output directory %s: %s", config.output_dir, error)
        return 1

    try:
        asyncio.run(run_snapshot_daemon(config))
    except KeyboardInterrupt:
        LOGGER.info("snapshot daemon stopped")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

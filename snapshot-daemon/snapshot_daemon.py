import asyncio
import logging

from agave_snapshot import AgaveLedgerSnapshotCreator, SnapshotOutcome
from finalized_epoch import FinalizedEpochPosition
from solana_rpc import SolanaRpcClient


BOUNDARY_SEARCH_SLOTS = 16
POLL_INTERVAL_SECONDS = 2

LOGGER = logging.getLogger("snapshot-daemon")


class EpochBoundarySnapshotDaemon:
    def __init__(
        self,
        rpc_client: SolanaRpcClient,
        snapshot_creator: AgaveLedgerSnapshotCreator,
    ) -> None:
        self._rpc_client = rpc_client
        self._snapshot_creator = snapshot_creator

    async def capture_snapshots_at_epoch_boundaries(self) -> None:
        observed_epoch_position = await self._rpc_client.fetch_finalized_epoch_position()
        LOGGER.info(
            "observed finalized epoch %d at slot %d; waiting for the next epoch",
            observed_epoch_position.epoch,
            observed_epoch_position.absolute_slot,
        )

        while True:
            finalized_epoch_after_transition = await self._wait_for_finalized_epoch_after(
                observed_epoch_position.epoch
            )
            LOGGER.info(
                "finalized epoch advanced from %d to %d",
                observed_epoch_position.epoch,
                finalized_epoch_after_transition.epoch,
            )
            await self._attempt_snapshot_at_previous_epoch_boundary(
                finalized_epoch_after_transition
            )

            observed_epoch_position = await self._rpc_client.fetch_finalized_epoch_position()
            LOGGER.info(
                "now observing finalized epoch %d at slot %d",
                observed_epoch_position.epoch,
                observed_epoch_position.absolute_slot,
            )

    async def _wait_for_finalized_epoch_after(
        self, observed_epoch: int
    ) -> FinalizedEpochPosition:
        while True:
            finalized_epoch_position = await self._rpc_client.fetch_finalized_epoch_position()
            if finalized_epoch_position.epoch > observed_epoch:
                return finalized_epoch_position
            await asyncio.sleep(POLL_INTERVAL_SECONDS)

    async def _attempt_snapshot_at_previous_epoch_boundary(
        self, finalized_epoch_position: FinalizedEpochPosition
    ) -> None:
        epoch_last_slot = finalized_epoch_position.previous_epoch_last_slot()
        LOGGER.info("searching for the boundary bank from slot %d", epoch_last_slot)

        boundary_bank_slot = await self._find_latest_finalized_block_slot(epoch_last_slot)
        if boundary_bank_slot is None:
            LOGGER.error(
                "no finalized block found from slot %d through slot %d",
                epoch_last_slot,
                epoch_last_slot - BOUNDARY_SEARCH_SLOTS + 1,
            )
            return

        LOGGER.info("selected epoch boundary bank at slot %d", boundary_bank_slot)
        snapshot_outcome = self._snapshot_creator.attempt_full_snapshot_at(
            boundary_bank_slot
        )
        match snapshot_outcome:
            case SnapshotOutcome.CREATED:
                LOGGER.info(
                    "agave-ledger-tool created the full snapshot at slot %d",
                    boundary_bank_slot,
                )
            case SnapshotOutcome.SKIPPED:
                LOGGER.info("snapshot at slot %d already exists", boundary_bank_slot)
            case SnapshotOutcome.FAILED:
                # The snapshot creator logs the process-specific failure details.
                return
            case SnapshotOutcome.INTERRUPTED:
                LOGGER.info(
                    "snapshot creation at slot %d was interrupted",
                    boundary_bank_slot,
                )

    async def _find_latest_finalized_block_slot(
        self, epoch_last_slot: int
    ) -> int | None:
        for candidate_slot in range(
            epoch_last_slot,
            epoch_last_slot - BOUNDARY_SEARCH_SLOTS,
            -1,
        ):
            if await self._rpc_client.finalized_block_exists(candidate_slot):
                return candidate_slot
            LOGGER.info("slot %d has no finalized block", candidate_slot)
        return None

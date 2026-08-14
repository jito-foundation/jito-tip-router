from __future__ import annotations

import asyncio
import logging

from solders.rpc.errors import (
    BlockNotAvailableMessage,
    LongTermStorageSlotSkippedMessage,
    SlotSkippedMessage,
)
from solders.transaction_status import TransactionDetails
from solana.rpc.async_api import AsyncClient
from solana.rpc.commitment import Finalized
from solana.rpc.core import RPCException

from finalized_epoch import FinalizedEpochPosition


RPC_TIMEOUT_SECONDS = 10
RPC_RETRY_SECONDS = 5

LOGGER = logging.getLogger("snapshot-daemon")


class SolanaRpcClient:
    def __init__(self, rpc_url: str) -> None:
        self._client = AsyncClient(
            rpc_url,
            commitment=Finalized,
            timeout=RPC_TIMEOUT_SECONDS,
        )

    async def close(self) -> None:
        await self._client.close()

    async def fetch_finalized_epoch_position(self) -> FinalizedEpochPosition:
        while True:
            try:
                epoch_info = (await self._client.get_epoch_info(Finalized)).value
                return FinalizedEpochPosition(
                    epoch=epoch_info.epoch,
                    absolute_slot=epoch_info.absolute_slot,
                    slot_index=epoch_info.slot_index,
                )
            except Exception as error:
                await self._log_retry("getEpochInfo", error)

    async def finalized_block_exists(self, slot: int) -> bool:
        while True:
            try:
                block = await self._client.get_block(
                    slot,
                    transaction_details=TransactionDetails.None_,
                    rewards=False,
                    commitment=Finalized,
                )
                return block.value is not None
            except RPCException as error:
                match error.args:
                    case (
                        BlockNotAvailableMessage()
                        | SlotSkippedMessage()
                        | LongTermStorageSlotSkippedMessage(),
                        *_,
                    ):
                        return False
                await self._log_retry(f"getBlock({slot})", error)
            except Exception as error:
                await self._log_retry(f"getBlock({slot})", error)

    @staticmethod
    async def _log_retry(operation: str, error: Exception) -> None:
        LOGGER.warning(
            "%s failed: %s; retrying in %d seconds",
            operation,
            error,
            RPC_RETRY_SECONDS,
        )
        await asyncio.sleep(RPC_RETRY_SECONDS)

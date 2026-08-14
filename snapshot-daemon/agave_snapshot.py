from __future__ import annotations

import logging
import shlex
import subprocess
import threading
from enum import StrEnum, auto
from pathlib import Path
from typing import TextIO


LOGGER = logging.getLogger("snapshot-daemon")


class SnapshotOutcome(StrEnum):
    CREATED = auto()
    SKIPPED = auto()
    FAILED = auto()
    INTERRUPTED = auto()


class AgaveLedgerSnapshotCreator:
    def __init__(
        self,
        ledger_tool_bin: Path,
        ledger_path: Path,
        output_dir: Path,
    ) -> None:
        self._ledger_tool_bin = ledger_tool_bin
        self._ledger_path = ledger_path
        self._output_dir = output_dir

    def attempt_full_snapshot_at(self, slot: int) -> SnapshotOutcome:
        command = [
            str(self._ledger_tool_bin),
            "--ledger",
            str(self._ledger_path),
            "create-snapshot",
            str(slot),
            str(self._output_dir),
        ]
        LOGGER.info("starting snapshot command: %s", shlex.join(command))

        try:
            ledger_tool_process = subprocess.Popen(
                command,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                encoding="utf-8",
                errors="replace",
                bufsize=1,
            )
        except OSError as error:
            LOGGER.error("failed to start agave-ledger-tool: %s", error)
            return SnapshotOutcome.FAILED

        assert ledger_tool_process.stdout is not None
        assert ledger_tool_process.stderr is not None
        stream_threads = [
            threading.Thread(
                target=self._log_ledger_tool_stream,
                args=(ledger_tool_process.stdout, "stdout"),
                daemon=True,
            ),
            threading.Thread(
                target=self._log_ledger_tool_stream,
                args=(ledger_tool_process.stderr, "stderr"),
                daemon=True,
            ),
        ]
        for stream_thread in stream_threads:
            stream_thread.start()

        exit_code = ledger_tool_process.wait()
        for stream_thread in stream_threads:
            stream_thread.join()

        if exit_code == 0:
            return SnapshotOutcome.CREATED

        LOGGER.error(
            "agave-ledger-tool failed for slot %d with exit code %d",
            slot,
            exit_code,
        )
        return SnapshotOutcome.FAILED

    @staticmethod
    def _log_ledger_tool_stream(stream: TextIO, stream_name: str) -> None:
        try:
            for line in stream:
                LOGGER.info(
                    "agave-ledger-tool %s: %s",
                    stream_name,
                    line.rstrip("\r\n"),
                )
        finally:
            stream.close()

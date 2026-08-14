from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class FinalizedEpochPosition:
    epoch: int
    absolute_slot: int
    slot_index: int

    def previous_epoch_last_slot(self) -> int:
        return self.absolute_slot - self.slot_index - 1

import { retakingLockHelps } from "../../shared/editing/index.ts";
import { codeOf } from "../../shared/errors.ts";
import type { LockState } from "./layout-lock.ts";

/**
 * What the save bar says went wrong. A refusal for the lock's sake (the editing time ran out, or
 * someone took over) is told by the note of the lost lock, with its way on, so the bar leaves it
 * out while that note shows: one event, one note.
 */
export function shownSaveError(error: unknown, lock: LockState): unknown {
  const forTheLock = retakingLockHelps(codeOf(error));
  return forTheLock && lock.phase === "lost" ? null : error;
}

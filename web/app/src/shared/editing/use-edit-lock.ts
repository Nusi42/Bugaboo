import {
  askToFinish,
  editAnyway,
  newEditorId,
  letGoOfEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
  type EditTarget,
} from "@bugaboo/api-client";
import { useEffect, useState, useSyncExternalStore } from "react";

import { api } from "../api.ts";
import { codeOf } from "../errors.ts";
import type { SessionDeps } from "./edit-lock-session.ts";
import { useLetGoWhenHidden } from "./let-go-when-hidden.ts";
import { EditLockController, type LockState } from "./edit-lock-state.ts";
import { lostReasonOf } from "./long-text-model.ts";

/** The calls of the lock on one thing, with the real client behind them. */
function depsOf(what: EditTarget): SessionDeps {
  const client = api.client;
  return {
    take: (editor) => takeEditLock(client, what, editor),
    editAnyway: (editor) => editAnyway(client, what, editor),
    askToFinish: () => askToFinish(client, what),
    heartbeat: (token) => sendHeartbeat(client, what, token),
    release: (token) => letGoOfEditLock(client, what, token),
    startHeartbeats,
    newEditorId,
  };
}

/** What a screen gets: the state, and what the person can do about somebody else's lock. */
export interface EditLock {
  readonly state: LockState;
  readonly retry: () => void;
  /**
   * A save is through, or was refused: a refusal for the lock's sake tells the lock first, so it is
   * let go before it is asked for again, which both end in.
   */
  readonly settled: (error: unknown) => void;
  readonly editAnyway: () => void;
  readonly askToFinish: () => void;
}

/**
 * The edit lock on a layout or a workflow, held while its editor is open. `what` is the thing the
 * lock is about; it is read once, when the editor opens.
 */
export function useEditLock(what: EditTarget): EditLock {
  const [lock] = useState(() => new EditLockController(depsOf(what)));
  const state = useSyncExternalStore(lock.subscribe, lock.snapshot);
  useEffect(() => {
    lock.start();
  }, [lock]);
  // The lock goes with a hidden page (closed, reloaded or cached) and is asked for again when the
  // page comes back from the cache: the one path every editor of a lock uses.
  useLetGoWhenHidden(lock.dispose, lock.start);
  return {
    state,
    retry: lock.start,
    settled: (error) => {
      const reason = lostReasonOf(codeOf(error));
      if (reason !== undefined) {
        lock.refused(reason);
      }
      lock.start();
    },
    editAnyway: lock.editAnyway,
    askToFinish: lock.askToFinish,
  };
}

import {
  askToFinish,
  editAnyway,
  newEditorId,
  releaseEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
} from "@bugaboo/api-client";
import { useEffect, useState, useSyncExternalStore } from "react";

import { api } from "../../shared/api.ts";
import type { SessionDeps } from "../../shared/editing/edit-lock-session.ts";
import { lostReasonOf } from "../../shared/editing/index.ts";
import { codeOf } from "../../shared/errors.ts";
import { whatOf } from "./layouts.ts";
import { LayoutLockController, type LockState } from "./layout-lock.ts";

/** The calls of the lock on one layout, with the real client behind them. */
function depsOf(layout: string): SessionDeps {
  const what = whatOf(layout);
  const client = api.client;
  return {
    take: (editor) => takeEditLock(client, what, editor),
    editAnyway: (editor) => editAnyway(client, what, editor),
    askToFinish: () => askToFinish(client, what),
    heartbeat: (token) => sendHeartbeat(client, what, token),
    release: (token) => releaseEditLock(client, what, token),
    startHeartbeats,
    newEditorId,
  };
}

/** What a screen gets: the state, and what the person can do about somebody else's lock. */
export interface LayoutLock {
  readonly state: LockState;
  readonly retry: () => void;
  /**
   * A save of the layout is through, or was refused: a refusal for the lock's sake tells the lock
   * first, so it is let go before it is asked for again, which both end in.
   */
  readonly settled: (error: unknown) => void;
  readonly editAnyway: () => void;
  readonly askToFinish: () => void;
}

/** The edit lock on a layout, held while the editor is open. */
export function useLayoutLock(layout: string): LayoutLock {
  const [lock] = useState(() => new LayoutLockController(depsOf(layout)));
  const state = useSyncExternalStore(lock.subscribe, lock.snapshot);
  useEffect(() => {
    lock.start();
    return lock.dispose;
  }, [lock]);
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

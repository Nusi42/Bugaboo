import {
  askToFinish,
  editAnyway,
  layoutEditPart,
  newEditorId,
  releaseEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
  type EditWhat,
} from "@bugaboo/api-client";
import { useEffect, useState, useSyncExternalStore } from "react";

import { api } from "../../shared/api.ts";
import { personName } from "../../shared/person.ts";
import { LayoutLockController, type LockDeps, type LockState } from "./layout-lock.ts";

/** What a layout is, for the lock: the thing being edited. */
function whatOf(layout: string): EditWhat {
  return { record: { kind: "layout", id: layout }, part: layoutEditPart };
}

/** The calls of the lock on one layout, with the real client behind them. */
function depsOf(layout: string): LockDeps {
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
    nameOf: personName,
  };
}

/** What a screen gets: the state, and what the person can do about somebody else's lock. */
export interface LayoutLock {
  readonly state: LockState;
  readonly retry: () => void;
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
  return { state, retry: lock.start, editAnyway: lock.editAnyway, askToFinish: lock.askToFinish };
}

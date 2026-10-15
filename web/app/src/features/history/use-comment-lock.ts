import {
  newEditorId,
  releaseEditLock,
  sendHeartbeat,
  startHeartbeats,
  takeEditLock,
  type EditWhat,
  type Schema,
} from "@bugaboo/api-client";
import { useEffect, useState } from "react";

import { api } from "../../shared/api.ts";

export type LockState =
  | { readonly status: "waiting" }
  | { readonly status: "held"; readonly held: Schema<"HeldLockBody"> }
  | { readonly status: "mine"; readonly token: string }
  | { readonly status: "failed"; readonly error: unknown };

/** The part of a comment that is edited under a lock: its text. */
function whatOf(comment: string): EditWhat {
  return { record: { kind: "comment", id: comment }, part: "text" };
}

/**
 * Takes the lock on the text of a comment when the editor opens, keeps it with the heartbeat while
 * the editor is open, and lets go of it when the editor closes (ADR-0019). To take it again, the
 * screen mounts the editor anew.
 */
export function useCommentLock(comment: string): LockState {
  const [state, setState] = useState<LockState>({ status: "waiting" });
  useEffect(() => {
    const what = whatOf(comment);
    let open = true;
    let token: string | null = null;
    let stop = (): void => undefined;
    takeEditLock(api.client, what, newEditorId())
      .then((outcome) => {
        if (outcome.status !== "taken") {
          setState({ status: "held", held: outcome });
          return;
        }
        const own = outcome.lock_token;
        token = own;
        if (open) {
          setState({ status: "mine", token: own });
          stop = startHeartbeats(() => sendHeartbeat(api.client, what, own));
        }
      })
      .catch((error: unknown) => {
        setState({ status: "failed", error });
      });
    return () => {
      open = false;
      stop();
      if (token !== null) {
        void releaseEditLock(api.client, what, token);
      }
    };
  }, [comment]);
  return state;
}

import type { Heartbeat, LockOutcome } from "@bugaboo/api-client";

import { EditLockSession, type SessionDeps, type SessionOptions } from "./edit-lock-session.ts";
import { stillHeld } from "./long-text-testing.ts";

/** A heartbeat that nobody started. */
function idle(): Promise<unknown> {
  return Promise.resolve();
}

/** A session whose asks are answered when the test says so. */
export function rig(options: Partial<SessionOptions> = {}) {
  const released: string[] = [];
  const answers: ((outcome: LockOutcome) => void)[] = [];
  const lost: [string, string | undefined][] = [];
  let beating = 0;
  let beat: () => Promise<unknown> = idle;
  let heartbeat: Heartbeat = stillHeld;
  const asked = (): Promise<LockOutcome> =>
    new Promise<LockOutcome>((answer) => {
      answers.push(answer);
    });
  const deps: SessionDeps = {
    take: asked,
    editAnyway: asked,
    askToFinish: () => Promise.resolve(),
    heartbeat: () => Promise.resolve(heartbeat),
    release: (token) => {
      released.push(token);
      return Promise.resolve();
    },
    startHeartbeats: (run) => {
      beating += 1;
      beat = run;
      return () => {
        beating -= 1;
      };
    },
    newEditorId: () => "editor-1",
  };
  const session = new EditLockSession(deps, {
    onRanOut: "lost",
    onLost: (reason, by) => {
      lost.push([reason, by?.shown === "name" ? by.id : undefined]);
    },
    ...options,
  });
  return {
    session,
    released,
    answers,
    lost,
    beating: () => beating,
    beat: () => beat(),
    answerWith: (next: Heartbeat) => {
      heartbeat = next;
    },
  };
}

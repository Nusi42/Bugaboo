import type { Heartbeat, LockOutcome } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { LayoutLockController, type LockDeps } from "./layout-lock.ts";

const taken: LockOutcome = {
  status: "taken",
  lock_token: "token-1",
  expires_at: "2026-10-13T10:00:00Z",
  lease_seconds: 60,
  heartbeat_seconds: 15,
};

const held: LockOutcome = {
  status: "held",
  holder: { shown: "name", id: "p1", name: "Ada" },
  since: "2026-10-13T09:59:00Z",
  yours_elsewhere: false,
  can_edit_anyway: true,
};

const stillHeld: Heartbeat = {
  lock: { status: "held", expires_at: "2026-10-13T10:01:00Z" },
  notices: [],
  draft_saved: false,
};
const ranOut: Heartbeat = { lock: { status: "ran_out" }, notices: [], draft_saved: false };

/** The calls of the lock, recorded; the first answer to asking for the lock is `first`. */
function rig(first: LockOutcome, beat: Heartbeat) {
  const released: string[] = [];
  const beats: (() => Promise<unknown>)[] = [];
  const deps: LockDeps = {
    take: () => Promise.resolve(first),
    editAnyway: () => Promise.resolve(taken),
    askToFinish: () => Promise.resolve(),
    heartbeat: () => Promise.resolve(beat),
    release: (token) => {
      released.push(token);
      return Promise.resolve();
    },
    startHeartbeats: (run) => {
      beats.push(run);
      return () => {
        beats.length = 0;
      };
    },
    newEditorId: () => "editor-1",
    nameOf: () => "Ada",
  };
  return { lock: new LayoutLockController(deps), released, beats };
}

/** Lets the answers of the doubles arrive. */
async function settle(): Promise<void> {
  await new Promise<void>((done) => {
    setTimeout(done, 0);
  });
}

describe("iss_17 the edit lock on a layout", () => {
  it("is mine when it is free, and is let go when the editor closes", async () => {
    const { lock, released } = rig(taken, stillHeld);
    lock.start();
    await settle();
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "token-1" });
    lock.dispose();
    expect(released).toEqual(["token-1"]);
  });

  it("says who is editing when somebody else holds it, and lets the person take it over", async () => {
    const { lock } = rig(held, stillHeld);
    lock.start();
    await settle();
    expect(lock.snapshot()).toEqual({
      phase: "held",
      name: "Ada",
      since: "2026-10-13T09:59:00Z",
      canEditAnyway: true,
    });
    lock.editAnyway();
    await settle();
    expect(lock.snapshot().phase).toBe("mine");
  });

  it("is lost when a heartbeat says the lease ran out, and stops beating", async () => {
    const { lock, beats, released } = rig(taken, ranOut);
    lock.start();
    await settle();
    const [beat] = beats;
    await beat?.();
    expect(lock.snapshot()).toEqual({ phase: "lost" });
    expect(beats).toEqual([]);
    expect(released).toEqual(["token-1"]);
  });
});

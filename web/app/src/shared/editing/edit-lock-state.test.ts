import type { Heartbeat, LockOutcome } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { LockServer } from "./lock-server-testing.ts";
import type { SessionDeps } from "./edit-lock-session.ts";
import { EditLockController } from "./edit-lock-state.ts";

const taken: LockOutcome = {
  kind: "taken",
  lock_token: "token-1",
  expires_at: "2026-10-13T10:00:00Z",
  lease_seconds: 60,
  heartbeat_seconds: 15,
};

const held: LockOutcome = {
  kind: "held-by-other",
  holder: { shown: "name", id: "p1", name: "Ada" },
  since: "2026-10-13T09:59:00Z",
  yours_elsewhere: false,
  can_edit_anyway: true,
};

const stillHeld: Heartbeat = {
  lock: { kind: "renewed", expires_at: "2026-10-13T10:01:00Z" },
  notices: [],
  draft_saved: false,
};
const ranOut: Heartbeat = {
  lock: { kind: "ran-out" },
  notices: [],
  draft_saved: false,
};

/** The calls of the lock, recorded; the first answer to asking for the lock is `first`. */
function rig(first: LockOutcome, beat: Heartbeat) {
  const released: string[] = [];
  const beats: (() => Promise<unknown>)[] = [];
  const deps: SessionDeps = {
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
  };
  return { lock: new EditLockController(deps), released, beats };
}

/** A lock that answers each `take` when the test says so, to start twice while one is in flight. */
function slowRig() {
  const released: string[] = [];
  let beats = 0;
  const answers: ((outcome: LockOutcome) => void)[] = [];
  const deps: SessionDeps = {
    take: () =>
      new Promise<LockOutcome>((answer) => {
        answers.push(answer);
      }),
    editAnyway: () => Promise.resolve(taken),
    askToFinish: () => Promise.resolve(),
    heartbeat: () => Promise.resolve(stillHeld),
    release: (token) => {
      released.push(token);
      return Promise.resolve();
    },
    startHeartbeats: () => {
      beats += 1;
      return () => {
        beats -= 1;
      };
    },
    newEditorId: () => "editor-1",
  };
  return {
    lock: new EditLockController(deps),
    released,
    answers,
    beats: () => beats,
  };
}

/** The calls of a lock that a test does not look at. */
const quiet = {
  editAnyway: () => Promise.resolve(taken),
  askToFinish: () => Promise.resolve(),
  heartbeat: () => Promise.resolve(stillHeld),
  newEditorId: () => "editor-1",
};

/** A lock whose release answers only when the test says so, and the order of its calls. */
function orderRig() {
  const calls: string[] = [];
  const answers: (() => void)[] = [];
  let beats = 0;
  const deps: SessionDeps = {
    ...quiet,
    take: () => {
      calls.push("take");
      return Promise.resolve(taken);
    },
    release: () => {
      calls.push("release");
      return new Promise<void>((done) => {
        answers.push(done);
      });
    },
    startHeartbeats: () => {
      beats += 1;
      return () => {
        beats -= 1;
      };
    },
  };
  return {
    lock: new EditLockController(deps),
    calls,
    answers,
  };
}

/** The way to stop a heartbeat that nothing started. */
function stopNothing(): void {
  // Nothing runs, so nothing to stop.
}

/** Lets the answers of the doubles arrive. */
async function settle(): Promise<void> {
  await new Promise<void>((done) => {
    setTimeout(done, 0);
  });
}

describe("iss_17 asking for the lock after one is held", () => {
  it("asks for the lock again only after the old token is let go of", async () => {
    // The server hands the same editor the same token, so a release that arrives after the new
    // ask would take the new lock away.
    const { lock, calls, answers } = orderRig();
    lock.start();
    await settle();
    lock.start();
    await settle();
    expect(calls).toEqual(["take", "release"]);
    answers.shift()?.();
    await settle();
    expect(calls).toEqual(["take", "release", "take"]);
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "token-1" });
  });
});

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
      held: {
        kind: "held-by-other",
        holder: { shown: "name", id: "p1", name: "Ada" },
        since: "2026-10-13T09:59:00Z",
        yours_elsewhere: false,
        can_edit_anyway: true,
      },
    });
    lock.editAnyway();
    await settle();
    expect(lock.snapshot().phase).toBe("mine");
  });

  it("is lost when a heartbeat says the lease ran out, and stops beating without a release", async () => {
    const { lock, beats, released } = rig(taken, ranOut);
    lock.start();
    await settle();
    const [beat] = beats;
    await beat?.();
    expect(lock.snapshot()).toEqual({ phase: "lost" });
    expect(beats).toEqual([]);
    expect(released).toEqual([]);
  });
});

describe("iss_17 asking for the lock more than once", () => {
  it("takes only the answer of the last ask when it is asked twice, and lets go of the other", async () => {
    const { lock, released, answers, beats } = slowRig();
    // Strict mode, a second *Try again*, or a retry after a save: asked while one is in flight.
    lock.start();
    lock.start();
    expect(answers).toHaveLength(2);
    const [first, second] = answers;
    second?.({ ...taken, lock_token: "token-2" });
    await settle();
    first?.({ ...taken, lock_token: "token-1" });
    await settle();
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "token-2" });
    expect(beats()).toBe(1);
    expect(released).toEqual(["token-1"]);
    lock.dispose();
    expect(beats()).toBe(0);
    expect(released).toEqual(["token-1", "token-2"]);
  });

  it("does not take a lock for an editor that was closed while the answer was on its way", async () => {
    const { lock, released, answers, beats } = slowRig();
    lock.start();
    lock.dispose();
    answers[0]?.(taken);
    await settle();
    expect(beats()).toBe(0);
    expect(released).toEqual(["token-1"]);
  });
});

describe("iss_17 a refused save of a layout", () => {
  it("lets the lock go before it is asked for again, so the next save goes through", async () => {
    const server = new LockServer();
    const lock = new EditLockController({
      ...quiet,
      take: server.take,
      release: server.release,
      startHeartbeats: () => stopNothing,
    });
    lock.start();
    await settle();
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "T" });
    // The layout changed meanwhile: the lock is let go, then asked for again.
    lock.refused("version-changed");
    lock.start();
    await settle();
    expect(server.calls).toEqual(["take", "release T"]);
    server.arrive();
    await settle();
    expect(server.calls).toEqual(["take", "release T", "take"]);
    expect(server.held).toBe("T");
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "T" });
  });

  it("asks again after a save that went through, when the server let go of the lock itself", async () => {
    const server = new LockServer();
    const lock = new EditLockController({
      ...quiet,
      take: server.take,
      release: server.release,
      startHeartbeats: () => stopNothing,
    });
    lock.start();
    await settle();
    lock.start();
    await settle();
    // The release of the old token is still on its way: nothing asks until it is through.
    expect(server.calls).toEqual(["take", "release T"]);
    server.arrive();
    await settle();
    expect(lock.snapshot()).toEqual({ phase: "mine", token: "T" });
    expect(server.held).toBe("T");
  });
});

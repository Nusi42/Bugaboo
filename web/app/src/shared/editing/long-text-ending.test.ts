import { describe, expect, it } from "vitest";

import { deferred, holding, inTurn, settle, stillHeld, taken, world } from "./long-text-testing.ts";

type Lock = ReturnType<typeof taken>;

describe("iss_17 a lock that arrives after the editing ended is let go and starts nothing", () => {
  it("is let go when the person cancelled while it was being taken", async () => {
    const lock = deferred<Lock>();
    const w = world({ take: () => lock.promise });
    w.editor.open();
    w.editor.type("Mine");
    w.editor.cancel();
    lock.resolve(taken("t1"));
    await settle();
    expect(w.log).toContain("release t1");
    expect(w.log).not.toContain("start-beats");
    expect(w.editor.snapshot().phase.kind).toBe("reading");
  });

  it("is let go when the screen went away while it was being taken", async () => {
    const lock = deferred<Lock>();
    const w = world({ take: () => lock.promise });
    w.editor.open();
    w.editor.type("Mine");
    w.editor.dispose();
    lock.resolve(taken("t1"));
    await settle();
    expect(w.log).toContain("release t1");
    expect(w.log).not.toContain("start-beats");
  });
});

describe("iss_17 nothing is started after the screen went away", () => {
  it("starts no watching when the screen went away while asking who is editing", async () => {
    const holder = deferred<typeof holding | null>();
    const w = world({ peek: () => holder.promise });
    w.editor.open();
    w.editor.dispose();
    holder.resolve(holding);
    await settle();
    expect(w.log).not.toContain("start-beats");
    expect(w.editor.snapshot().phase.kind).toBe("open");
  });

  it("is let go when the screen went away while the lock was taken again", async () => {
    const second = deferred<Lock>();
    const locks = inTurn(Promise.resolve(taken("t1")), second.promise);
    const w = world({
      heartbeat: () => Promise.resolve({ ...stillHeld, lock: { kind: "ran-out" } }),
      take: () => locks(),
    });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    const beat = w.beat();
    await settle();
    w.editor.dispose();
    second.resolve(taken("t3"));
    await beat;
    expect(w.log).toContain("release t3");
    expect(w.log.filter((entry) => entry === "start-beats")).toHaveLength(1);
  });
});

describe("iss_05 a page that comes back from the cache after its lock was let go", () => {
  it("says the editing time ran out and keeps the text, so Try again can take the lock again", async () => {
    const w = world({ take: () => Promise.resolve(taken("t1")) });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    // The page was hidden: the lock went with it. It is shown again as it was left.
    w.editor.dispose();
    w.editor.cameBack();
    expect(w.log).toContain("release t1");
    const state = w.editor.snapshot();
    expect(state.phase).toEqual({ kind: "lost", reason: "ran-out" });
    expect(state.text).toBe("Mine");
  });

  it("leaves a box that was not being edited as it is", () => {
    const w = world({});
    w.editor.cameBack();
    expect(w.editor.snapshot().phase.kind).toBe("reading");
  });
});

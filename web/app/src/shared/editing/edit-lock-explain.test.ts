import { describe, expect, it } from "vitest";

import { rig } from "./edit-lock-testing.ts";
import { ben, settle, taken } from "./long-text-testing.ts";

const tookOver = {
  lock: { status: "taken-over", by: ben },
  notices: [],
  draft_saved: false,
} as const;

describe("iss_17 asking why a save was refused leaves no lock and no beat behind (M3 review)", () => {
  it("stops the regular beats first, so a beat that lands later says nothing", async () => {
    const { session, answers, beating, beat, answerWith, lost } = rig();
    const asking = session.take();
    answers[0]?.(taken("T"));
    await asking;
    answerWith(tookOver);
    expect(await session.explain("ran-out")).toEqual({ reason: "taken-over", by: ben });
    expect(beating()).toBe(0);
    expect(session.token).toBeUndefined();
    // A regular beat that was already on its way lands now: the session is done with that lock.
    await beat();
    await settle();
    expect(lost).toEqual([]);
  });

  it("tells who asked to finish, from the notice of that one heartbeat", async () => {
    const asked: unknown[] = [];
    const { session, answers, answerWith } = rig({
      onAsked: (from) => {
        asked.push(from);
      },
    });
    const asking = session.take();
    answers[0]?.(taken("T"));
    await asking;
    answerWith({
      ...tookOver,
      notices: [{ kind: "finish-asked", from: ben, at: "2026-10-10T10:00:00Z" }],
    });
    await session.explain("ran-out");
    expect(asked).toEqual([ben]);
  });
});

describe("iss_17 a ran-out that takes the lock again is overtaken by a refused save (M3 review)", () => {
  it("does not leave a lock that a retaking ran-out asks for after the person gave up", async () => {
    const { session, answers, released, beating, beat, answerWith } = rig({
      onRanOut: "take-again",
    });
    const asking = session.take();
    answers[0]?.(taken("T"));
    await asking;
    answerWith({ lock: { status: "ran-out" }, notices: [], draft_saved: false });
    const retaking = beat();
    await settle();
    // The lock is asked for again; meanwhile a refused save asks why and gives the lock up.
    await session.explain("ran-out");
    answers[1]?.(taken("T2"));
    await retaking;
    await settle();
    expect(session.token).toBeUndefined();
    expect(beating()).toBe(0);
    expect(released).toContain("T2");
  });
});

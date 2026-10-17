import { ApiError, draftKeptCodes, type ErrorCode } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  ben,
  heldBy,
  inTurn,
  refusal,
  refusedOnce,
  settle,
  stillHeld,
  taken,
  world,
} from "./long-text-testing.ts";
import { lostReasonOf } from "./long-text-model.ts";

/** A text that is typed, with a save that the server refuses with this code. */
async function refusedWith(code: ErrorCode) {
  const w = world({ save: () => Promise.reject(refusal(code)) });
  w.editor.open();
  w.editor.type("My words");
  await settle();
  await w.editor.save();
  return w;
}

describe("iss_17 every refusal that keeps a draft has a reason", () => {
  it("is known for each code of the list of the server", () => {
    for (const code of draftKeptCodes) {
      expect(lostReasonOf(code)).toBeDefined();
    }
    expect(lostReasonOf("resource.not_found")).toBeUndefined();
  });
});

describe("iss_17 a refused save tells why the lock is gone", () => {
  it("says who took over when the server says so after a save was refused for a lost lock (M3 review)", async () => {
    // Ben used *Edit anyway* and saved: the lock is gone, and only the note for this person is left.
    const tookOver = { ...stillHeld, lock: { status: "taken_over", by: ben } } as const;
    const w = world({
      save: () => Promise.reject(refusal("edit.lock_lost")),
      heartbeat: () => Promise.resolve(tookOver),
    });
    w.editor.open();
    w.editor.type("My words");
    await settle();
    await w.editor.save();
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "taken-over", by: ben });
    expect(w.editor.snapshot().text).toBe("My words");
  });

  it("names who took over from the refusal itself, with no heartbeat (M3-F176, F180)", async () => {
    const w = world({
      save: () => Promise.reject(refusal("edit.taken_over", { taken_over_by: ben })),
      heartbeat: () => Promise.reject(new Error("no heartbeat is needed")),
    });
    w.editor.open();
    w.editor.type("My words");
    await settle();
    await w.editor.save();
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "taken-over", by: ben });
  });
});

describe("iss_17 nothing is lost when the save is refused", () => {
  it("says the editing time ran out and keeps the text", async () => {
    const w = await refusedWith("edit.lock_lost");
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "ran-out" });
    expect(w.editor.snapshot().text).toBe("My words");
    expect(w.log).toContain("stop-beats");
  });

  it("says somebody took the editing over", async () => {
    const w = await refusedWith("edit.taken_over");
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "taken-over" });
  });

  it("says the text changed since and reads the record again for Compare and save", async () => {
    const w = await refusedWith("edit.version_changed");
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "version-changed" });
    expect(w.log).toContain("refresh");
  });

  it("stays locked, with the text, when the refusal is not the lock's business", async () => {
    const w = world({ save: () => Promise.reject(new ApiError(0, undefined)) });
    w.editor.open();
    w.editor.type("My words");
    await settle();
    await w.editor.save();
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    expect(w.editor.snapshot().problem).toBeInstanceOf(ApiError);
  });

  it("saves the chosen text on the newest version after Compare and save", async () => {
    const w = world({
      save: refusedOnce("edit.version_changed", (input) => {
        w.saves.push(input);
      }),
    });
    w.editor.open();
    w.editor.type("My words");
    await settle();
    await w.editor.save();
    w.editor.setSource({ saved: "Ben's words", version: 4 });
    await w.editor.resolve("Mine and Ben's");
    expect(w.saves.at(-1)).toEqual({ text: "Mine and Ben's", version: 4, lockToken: "t1" });
    expect(w.editor.snapshot().phase.kind).toBe("reading");
  });
});

/** What the editor did, in order, after a save that the server refused with this code. */
async function logAfter(code: ErrorCode): Promise<readonly string[]> {
  return (await refusedWith(code)).log;
}

describe("iss_17 a refused save lets go of the lock only when the lock is still the person's", () => {
  it("lets go when the text changed since, so nobody waits for the lease to run out", async () => {
    expect(await logAfter("edit.version_changed")).toContain("release t1");
  });

  it("does not let go of a lock that ran out or was taken over: it is not the person's any more", async () => {
    const log = [...(await logAfter("edit.lock_lost")), ...(await logAfter("edit.taken_over"))];
    expect(log.some((line) => line.startsWith("release"))).toBe(false);
  });
});

describe("iss_17 what the heartbeat brings back", () => {
  it("tells that somebody asked to finish", async () => {
    const w = world({
      heartbeat: () =>
        Promise.resolve({
          ...stillHeld,
          notices: [{ kind: "finish_asked", from: ben, at: "2026-10-10T10:00:00Z" }],
        }),
    });
    w.editor.open();
    w.editor.type("x");
    await settle();
    await w.beat();
    expect(w.editor.snapshot().asked).toEqual(ben);
  });
});

describe("iss_17 what the heartbeat brings back about the lock", () => {
  it("keeps the text when somebody took the editing over", async () => {
    const w = world({
      heartbeat: () => Promise.resolve({ ...stillHeld, lock: { status: "taken_over", by: ben } }),
    });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    await w.beat();
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "taken-over", by: ben });
    expect(w.editor.snapshot().text).toBe("Mine");
  });

  it("takes the lock again when the lease ran out and nobody has it", async () => {
    const answers = inTurn({ ...stillHeld, lock: { status: "ran_out" } } as const, stillHeld);
    const locks = inTurn(taken("t1"), taken("t3"));
    const w = world({
      heartbeat: () => Promise.resolve(answers()),
      take: () => Promise.resolve(locks()),
    });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    await w.beat();
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    await w.editor.save();
    expect(w.saves[0]?.lockToken).toBe("t3");
  });

  it("says the time ran out when somebody else has the lock by then", async () => {
    const locks = inTurn(taken("t1"), heldBy(false));
    const w = world({
      heartbeat: () => Promise.resolve({ ...stillHeld, lock: { status: "ran_out" } }),
      take: () => Promise.resolve(locks()),
    });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    await w.beat();
    expect(w.editor.snapshot().phase).toEqual({ kind: "lost", reason: "ran-out", by: ben });
    expect(w.editor.snapshot().text).toBe("Mine");
  });
});

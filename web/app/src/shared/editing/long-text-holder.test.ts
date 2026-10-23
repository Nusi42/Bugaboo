import { ApiError } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  deferred,
  heldBy,
  holding,
  inTurn,
  settle,
  stillHeld,
  taken,
  world,
} from "./long-text-testing.ts";

describe("iss_17 the box and the lock", () => {
  it("keeps the text and offers Compare and save when the lock cannot be taken again", async () => {
    const locks = inTurn(
      () => Promise.resolve(taken("t1")),
      () => Promise.reject(new ApiError(0, undefined)),
    );
    const w = world({
      heartbeat: () => Promise.resolve({ ...stillHeld, lock: { status: "ran-out" } }),
      take: () => locks()(),
    });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    await w.beat();
    const state = w.editor.snapshot();
    expect(state.phase).toEqual({ kind: "lost", reason: "ran-out" });
    expect(state.problem).toBeInstanceOf(ApiError);
    expect(state.text).toBe("Mine");
  });

  it("takes every key typed while the lock is being asked for", async () => {
    const lock = deferred<ReturnType<typeof taken>>();
    const w = world({ take: () => lock.promise });
    w.editor.open();
    for (const text of ["N", "Ne", "New", "New words"]) {
      w.editor.type(text);
    }
    expect(w.editor.snapshot().phase.kind).toBe("taking");
    expect(w.editor.snapshot().text).toBe("New words");
    lock.resolve(taken("t1"));
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("locked");
    expect(w.editor.snapshot().text).toBe("New words");
    expect(w.kept.at(-1)?.text).toBe("New words");
  });
});

describe("iss_17 Save pressed while the lock is being asked for", () => {
  it("saves when the lock comes, if Save was pressed while it was being asked for", async () => {
    const lock = deferred<ReturnType<typeof taken>>();
    const w = world({ take: () => lock.promise });
    w.editor.open();
    w.editor.type("New words");
    await w.editor.save();
    expect(w.saves).toEqual([]);
    expect(w.editor.snapshot().phase.kind).toBe("taking");
    lock.resolve(taken("t1"));
    await settle();
    expect(w.saves).toEqual([{ text: "New words", version: 3, lockToken: "t1" }]);
    expect(w.editor.snapshot().phase.kind).toBe("reading");
  });

  it("does not save later on its own when the lock was not given after Save was pressed", async () => {
    const lock = deferred<ReturnType<typeof heldBy>>();
    const w = world({ take: () => lock.promise });
    w.editor.open();
    w.editor.type("New words");
    await w.editor.save();
    lock.resolve(heldBy(true));
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("held");
    expect(w.saves).toEqual([]);
  });
});

describe("ux_06 the box learns who is editing", () => {
  it("tells at once who is editing when the box opens, and opens when they let go", async () => {
    const holders = inTurn(holding, holding, null);
    const w = world({ peek: () => Promise.resolve(holders()) });
    w.editor.open();
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("held");
    await w.beat();
    expect(w.editor.snapshot().phase.kind).toBe("held");
    await w.beat();
    expect(w.editor.snapshot().phase).toEqual({ kind: "open" });
    expect(w.log).not.toContain("take e1");
  });

  it("leaves a person who already typed with their text when somebody is found later", async () => {
    const holder = deferred<typeof holding | null>();
    const w = world({ peek: () => holder.promise });
    w.editor.open();
    w.editor.type("Typed first");
    holder.resolve(holding);
    await settle();
    expect(w.editor.snapshot().text).toBe("Typed first");
    expect(w.editor.snapshot().phase.kind).not.toBe("held");
  });

  it("is told by the answer to taking the lock when somebody else holds it", async () => {
    const w = world({ take: () => Promise.resolve(heldBy(true)) });
    w.editor.open();
    w.editor.type("Mine");
    await settle();
    expect(w.editor.snapshot().phase.kind).toBe("held");
    expect(w.editor.snapshot().text).toBe("Mine");
  });
});

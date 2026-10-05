import { QueryClient } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";

import {
  optimisticSend,
  saveStateOf,
  skeletonDelayMs,
  startDelay,
  undoWindowMs,
  type OptimisticChange,
  type Schedule,
} from "./optimistic.ts";
import { createUndoStore } from "./optimistic-undo.ts";

interface Names {
  readonly names: readonly string[];
}

const key = ["names"] as const;

/** A change that adds a name to the list in the cache; `send` is whatever the test needs. */
function addName(send: (variables: { readonly name: string }) => Promise<string>) {
  const change: OptimisticChange<{ readonly name: string }, Names, string> = {
    queryKey: key,
    apply: (before, { name }) => ({ names: [...(before?.names ?? []), name] }),
    send,
  };
  return change;
}

/** A promise that the test settles when it chooses: a slow server. */
function slowServer() {
  const { promise, resolve, reject } = Promise.withResolvers<string>();
  return { promise, answer: resolve, refuse: reject };
}

/** Lets what was started run as far as it can without an answer from the server. */
function nextTick(): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, 0);
  });
}

/** What an undo does when the test does not look at it. */
const doNothing = (): void => {
  // Nothing to take back in this test.
};

/** Timers that only move when the test says so. */
function manualClock() {
  let now = 0;
  const waiting: { at: number; run: () => void; stopped: boolean }[] = [];
  const schedule: Schedule = (run, ms) => {
    const entry = { at: now + ms, run, stopped: false };
    waiting.push(entry);
    return () => {
      entry.stopped = true;
    };
  };
  return {
    schedule,
    advance: (ms: number) => {
      now += ms;
      for (const entry of waiting) {
        if (!entry.stopped && entry.at <= now) {
          entry.stopped = true;
          entry.run();
        }
      }
    },
  };
}

describe("ux_06 a change shows at once", () => {
  it("is in the cache within 100 ms, while the slow server has not answered", async () => {
    const client = new QueryClient();
    client.setQueryData<Names>(key, { names: ["Ana"] });
    const server = slowServer();
    const run = optimisticSend(
      () => client,
      addName(() => server.promise),
    );

    const started = performance.now();
    const done = run({ name: "Ben" });
    await nextTick();
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
    expect(performance.now() - started).toBeLessThan(100);

    server.answer("ok");
    await expect(done).resolves.toBe("ok");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);
  });

  it("goes back to the old data when the server refuses, and passes the error on", async () => {
    const client = new QueryClient();
    client.setQueryData<Names>(key, { names: ["Ana"] });
    const server = slowServer();
    const run = optimisticSend(
      () => client,
      addName(() => server.promise),
    );

    const done = run({ name: "Ben" });
    await nextTick();
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana", "Ben"]);

    server.refuse(new Error("refused"));
    await expect(done).rejects.toThrow("refused");
    expect(client.getQueryData<Names>(key)?.names).toEqual(["Ana"]);
  });

  it("says Saving, Saved or Not saved yet for each stage of a change", () => {
    expect(saveStateOf("idle")).toBe("idle");
    expect(saveStateOf("pending")).toBe("saving");
    expect(saveStateOf("success")).toBe("saved");
    expect(saveStateOf("error")).toBe("not-saved");
  });
});

describe("ux_06 skeleton shapes wait for 0.3 seconds", () => {
  it("do not show before the delay and not at all when the answer came first", () => {
    const clock = manualClock();
    let shown = false;
    const stop = startDelay(
      skeletonDelayMs,
      () => {
        shown = true;
      },
      clock.schedule,
    );
    clock.advance(skeletonDelayMs - 1);
    expect(shown).toBe(false);
    stop();
    clock.advance(skeletonDelayMs);
    expect(shown).toBe(false);
  });

  it("show once the delay has passed", () => {
    const clock = manualClock();
    let shown = false;
    startDelay(
      skeletonDelayMs,
      () => {
        shown = true;
      },
      clock.schedule,
    );
    clock.advance(skeletonDelayMs);
    expect(shown).toBe(true);
  });
});

describe("ux_06 undo for 10 seconds", () => {
  it("takes an action back while it is on offer, once", () => {
    const clock = manualClock();
    const store = createUndoStore(clock.schedule);
    let undone = 0;
    const id = store.offer({
      message: "Done",
      undo: () => {
        undone += 1;
      },
    });
    expect(store.list().map((offer) => offer.message)).toEqual(["Done"]);
    clock.advance(undoWindowMs - 1);
    expect(store.take(id)).toBe(true);
    expect(undone).toBe(1);
    expect(store.list()).toEqual([]);
    expect(store.take(id)).toBe(false);
    expect(undone).toBe(1);
  });

  it("stops offering after the 10 seconds", () => {
    const clock = manualClock();
    const store = createUndoStore(clock.schedule);
    let undone = 0;
    const id = store.offer({
      message: "Done",
      undo: () => {
        undone += 1;
      },
    });
    clock.advance(undoWindowMs);
    expect(store.list()).toEqual([]);
    expect(store.take(id)).toBe(false);
    expect(undone).toBe(0);
  });

  it("tells those who show the offers about every change", () => {
    const store = createUndoStore(manualClock().schedule);
    let heard = 0;
    const stop = store.subscribe(() => {
      heard += 1;
    });
    const id = store.offer({ message: "Done", undo: doNothing });
    store.take(id);
    expect(heard).toBe(2);
    stop();
    store.offer({ message: "Again", undo: doNothing });
    expect(heard).toBe(2);
  });
});

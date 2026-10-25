import { describe, expect, it } from "vitest";

import { undoWindowMs } from "./optimistic.ts";
import { createUndoStore } from "./optimistic-undo.ts";
import { doNothing } from "./do-nothing.ts";
import { manualClock } from "./optimistic-testing.ts";

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

/** A change per name: each one has its own slow server, so tests can answer them in any order. */
describe("ux_06 Undo waits while it is looked at", () => {
  it("stops counting while the offer is pointed at or has the focus, and goes on after", () => {
    const clock = manualClock();
    const store = createUndoStore(clock.schedule, clock.now);
    let undone = 0;
    const id = store.offer({
      message: "Done",
      undo: () => {
        undone += 1;
      },
    });
    clock.advance(6000);
    store.pause(id);
    clock.advance(60_000);
    expect(store.list()).toHaveLength(1);
    store.resume(id);
    clock.advance(3999);
    expect(store.list()).toHaveLength(1);
    clock.advance(2);
    expect(store.list()).toEqual([]);
    expect(undone).toBe(0);
  });

  it("can still be taken while it is held", () => {
    const clock = manualClock();
    const store = createUndoStore(clock.schedule, clock.now);
    let undone = 0;
    const id = store.offer({
      message: "Done",
      undo: () => {
        undone += 1;
      },
    });
    store.pause(id);
    expect(store.take(id)).toBe(true);
    expect(undone).toBe(1);
  });

  it("offers the latest action to the shortcut", () => {
    const store = createUndoStore(manualClock().schedule, () => 0);
    const first = store.offer({ message: "One", undo: doNothing });
    const second = store.offer({ message: "Two", undo: doNothing });
    expect(store.latest()?.id).toBe(second);
    store.take(second);
    expect(store.latest()?.id).toBe(first);
  });
});

describe("a notice in the place of the offers", () => {
  it("shows a notice that has nothing to undo, and the shortcut skips it", () => {
    const store = createUndoStore(manualClock().schedule, () => 0);
    const first = store.offer({ message: "One", undo: doNothing });
    const notice = store.offer({ message: "It was not brought back" });
    expect(store.list().map((each) => each.id)).toEqual([first, notice]);
    expect(store.latest()?.id).toBe(first);
    expect(store.take(notice)).toBe(true);
    expect(store.list().map((each) => each.id)).toEqual([first]);
  });
});

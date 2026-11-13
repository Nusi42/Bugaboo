import { describe, expect, it } from "vitest";

import { skeletonDelayMs } from "./after-delay.ts";
import { windowState } from "./props.ts";
import { applyWindowSize, resizeByKey, type ResizeKey } from "./use-window-resize.ts";
import {
  delayLeft,
  handoverFor,
  offerHandover,
  withdrawHandover,
  type Handover,
} from "./window-presence.ts";
import { edgesOf, notBelow, windowStand } from "./window-size.ts";
import { keyboardStep, type WindowElement, type WindowSize } from "./window-size.ts";
import {
  firstShape,
  dragEdgesOf,
  hiddenPresence,
  isWaiting,
  phaseOf,
  presenceAfter,
  reduceShape,
  shapeAction,
  windowPhase,
  type PresenceFacts,
  type WindowPhase,
  type WindowPresence,
} from "./window-shape.ts";

/** A window element that remembers what was set on it, and is as big as `size`. */
function fakeWindow(size: WindowSize): { element: WindowElement; set: [string, string][] } {
  const set: [string, string][] = [];
  const element: WindowElement = {
    getBoundingClientRect: () => size,
    style: {
      setProperty: (name: string, value: string) => {
        set.push([name, value]);
      },
    },
  };
  return { element, set };
}

const nobody = (): void => {
  /* nobody listens */
};
const key = { ctrl: true, alt: true, right: "ArrowRight", enter: "Enter" } as const;

function press(keyName: string, ctrlKey = true): ResizeKey & { prevented: boolean } {
  const event = {
    ctrlKey,
    altKey: true,
    key: keyName,
    prevented: false,
    preventDefault: () => {
      event.prevented = true;
    },
  };
  return event;
}

describe("ux_6_3 a size reaches the page as two variables of the theme file and nothing else", () => {
  it("sets only the width and the height variables", () => {
    const { element, set } = fakeWindow({ width: 1, height: 1 });
    applyWindowSize(element, { width: 800, height: 600 });
    expect(set).toEqual([
      ["--window-width", "800px"],
      ["--window-height", "600px"],
    ]);
  });
});

describe("ux_6_3 a window can be minimized, restored and maximized", () => {
  it("minimizes and restores without touching the maximize", () => {
    const maximized = reduceShape(firstShape, shapeAction.toggleMaximize);
    const parked = reduceShape(maximized, shapeAction.minimize);
    expect(parked).toEqual({ minimized: true, maximized: true });
    expect(reduceShape(parked, shapeAction.restore)).toEqual({ minimized: false, maximized: true });
  });

  it("toggles maximize and takes it back with Restore down", () => {
    const once = reduceShape(firstShape, shapeAction.toggleMaximize);
    expect(once.maximized).toBe(true);
    expect(reduceShape(once, shapeAction.toggleMaximize).maximized).toBe(false);
  });

  it("opens again as first drawn after it was closed", () => {
    const busy = reduceShape(
      reduceShape(firstShape, shapeAction.toggleMaximize),
      shapeAction.minimize,
    );
    expect(reduceShape(busy, shapeAction.closed)).toEqual(firstShape);
  });
});

describe("ux_04 Ctrl and Alt with an arrow key resize a window by a step", () => {
  it("makes it wider by a step, sets the variables and tells the new size", () => {
    const { element, set } = fakeWindow({ width: 760, height: 500 });
    const told: WindowSize[] = [];
    const event = press(key.right);
    resizeByKey(element, event, (size) => {
      told.push(size);
    });
    expect(set[0]).toEqual(["--window-width", `${760 + keyboardStep}px`]);
    expect(event.prevented).toBe(true);
    expect(told).toHaveLength(1);
  });

  it("leaves other keys, and arrows without Ctrl, alone", () => {
    const { element, set } = fakeWindow({ width: 760, height: 500 });
    const other = press(key.enter);
    const plain = press(key.right, false);
    resizeByKey(element, other, nobody);
    resizeByKey(element, plain, nobody);
    expect(set).toEqual([]);
    expect(other.prevented).toBe(false);
    expect(plain.prevented).toBe(false);
  });
});

describe("ux_6_3 a window that is as tall as its content is not made taller by making it shorter", () => {
  it("holds the smallest height, but not above the height the window began with", () => {
    expect(notBelow({ width: 500, height: 100 }, 360, 700)).toEqual({ width: 500, height: 360 });
    expect(notBelow({ width: 500, height: 150 }, 360, 220)).toEqual({ width: 500, height: 220 });
    expect(notBelow({ width: 500, height: 900 }, 360, 700)).toEqual({ width: 500, height: 900 });
  });
});

/** The presence of a window after each of these moments, one after the other, from a closed window. */
function presencesThrough(moments: readonly PresenceFacts[]): WindowPresence[] {
  const seen: WindowPresence[] = [];
  moments.reduce((before, facts) => {
    const after = presenceAfter(before, facts);
    seen.push(after);
    return after;
  }, hiddenPresence);
  return seen;
}

/** Whether the window is shown after each moment. */
function shownThrough(moments: readonly PresenceFacts[]): boolean[] {
  return presencesThrough(moments).map((presence) => presence.shown);
}

/** How the window stands after each moment. */
function phasesThrough(moments: readonly PresenceFacts[]): WindowPhase[] {
  const presences = presencesThrough(moments);
  return moments.map((facts, at) => phaseOf(presences[at] ?? hiddenPresence, facts.late));
}

const open = { isOpen: true, late: false } as const;
const closed = { isOpen: false, ready: false, late: false } as const;

describe("ux_6_1 a window is shown once its content is there and then stays shown", () => {
  it("stays shown when its content is on its way again (another project picked)", () => {
    const moments = [
      { ...open, ready: false },
      { ...open, ready: true },
      { ...open, ready: false },
    ];
    expect(shownThrough(moments)).toEqual([false, true, true]);
    expect(phasesThrough(moments).at(-1)).toBe(windowPhase.middle);
  });

  it("starts hidden again after it closes and opens", () => {
    expect(shownThrough([{ ...open, ready: true }, closed, { ...open, ready: false }])).toEqual([
      true,
      false,
      false,
    ]);
  });

  it("is shown in its first frame, in the middle, when its content is there at once", () => {
    expect(presencesThrough([{ ...open, ready: true }])).toEqual([
      { shown: true, anchored: false },
    ]);
    expect(phasesThrough([{ ...open, ready: true }])).toEqual([windowPhase.middle]);
  });

  it("shows nothing for the first 0.3 s, then grey shapes, and keeps their top when the content arrives", () => {
    expect(
      phasesThrough([
        { ...open, ready: false },
        { isOpen: true, ready: false, late: true },
        { ...open, ready: true },
      ]),
    ).toEqual([windowPhase.quiet, windowPhase.loading, windowPhase.top]);
  });
});

describe("ux_6_1 a window that waits is not resized, and one at the top has no top edge", () => {
  it("waits while nothing or grey shapes show, and not once its content is there", () => {
    expect(Object.values(windowPhase).map((phase) => [phase, isWaiting(phase)])).toEqual([
      [windowPhase.quiet, true],
      [windowPhase.loading, true],
      [windowPhase.top, false],
      [windowPhase.middle, false],
    ]);
  });

  it("drags at every edge in the middle, and at all but the top ones at the top", () => {
    expect(edgesOf(windowStand.middle)).toHaveLength(8);
    expect([...edgesOf(windowStand.top)].toSorted()).toEqual(["e", "s", "se", "sw", "w"]);
  });

  it("goes on showing grey shapes when it takes over from a stand-in that showed them", () => {
    expect(phaseOf({ shown: false, anchored: true }, false)).toBe(windowPhase.loading);
    expect(phaseOf({ shown: false, anchored: false }, false)).toBe(windowPhase.quiet);
  });

  it("has no places to drag when it is maximized, a full screen or waiting", () => {
    expect(dragEdgesOf(windowState.maximized, windowPhase.middle)).toEqual([]);
    expect(dragEdgesOf(windowState.full, windowPhase.middle)).toEqual([]);
    expect(dragEdgesOf(windowState.normal, windowPhase.loading)).toEqual([]);
    expect(dragEdgesOf(windowState.normal, windowPhase.middle)).toEqual(
      edgesOf(windowStand.middle),
    );
    expect(dragEdgesOf(windowState.normal, windowPhase.top)).toEqual(edgesOf(windowStand.top));
  });
});

/** What a waiting window of `form` offers, at the top. */
function waitingAt(form: string): Handover {
  return { form, presence: { shown: false, anchored: true }, returnTo: null, since: 0 };
}

describe("ux_6_1 a window that waits for its code hands over only to a window of its form", () => {
  it("is taken over by the same form, not by another, and only its own offer is taken back", () => {
    const newIssue = waitingAt("new-issue");
    const timeEntry = waitingAt("time-entry");
    offerHandover(newIssue);
    expect(handoverFor("new-issue")).toBe(newIssue);
    expect(handoverFor("time-entry")).toBeNull();
    offerHandover(timeEntry);
    withdrawHandover(newIssue);
    expect(handoverFor("time-entry")).toBe(timeEntry);
    withdrawHandover(timeEntry);
    expect(handoverFor("time-entry")).toBeNull();
  });

  it("lets the window that takes over wait only for what is left of the 0.3 s", () => {
    expect(delayLeft(1000, 1000)).toBe(skeletonDelayMs);
    expect(delayLeft(1000, 1100)).toBe(skeletonDelayMs - 100);
    expect(delayLeft(1000, 1000 + skeletonDelayMs + 50)).toBe(0);
  });
});

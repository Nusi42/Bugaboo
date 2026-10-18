import { describe, expect, it } from "vitest";

import { applyWindowSize, resizeByKey, type ResizeKey } from "./use-window-resize.ts";
import { notBelow } from "./window-size.ts";
import { keyboardStep, type WindowElement, type WindowSize } from "./window-size.ts";
import { firstShape, reduceShape, shapeAction } from "./window-shape.ts";

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

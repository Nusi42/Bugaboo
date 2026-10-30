import { beforeEach, describe, expect, it } from "vitest";

import {
  isLeavingPage,
  isModuleLoadError,
  resetLeavingPage,
  shouldReport,
  watchLeavingPage,
  type PageChange,
} from "./leaving-page.ts";

/** A window that can be told what happened, and a clock that runs when it is told to. */
function watched() {
  const heard = new Map<string, (event: PageChange) => void>();
  const timers = new Map<number, () => void>();
  let next = 0;
  watchLeavingPage(
    {
      addEventListener: (type, listener) => {
        heard.set(type, listener);
      },
    },
    {
      after: (run) => {
        next += 1;
        timers.set(next, run);
        return next;
      },
      cancel: (timer) => {
        timers.delete(timer);
      },
    },
  );
  return {
    tell: (type: string, persisted = false) => {
      heard.get(type)?.({ persisted });
    },
    timers,
  };
}

beforeEach(() => {
  resetLeavingPage();
});

describe("test_connections which errors the app reports while a page is being left", () => {
  const failed = new TypeError("Importing a module script failed.");
  const other = new TypeError("Cannot read properties of undefined");
  it.each([
    ["a failed load while leaving", failed, true, false],
    ["another error while leaving", other, true, true],
    ["a failed load while staying", failed, false, true],
    ["another error while staying", other, false, true],
  ])("%s", (_name, error: Readonly<Error>, leaving, reported) => {
    expect(shouldReport(error, leaving)).toBe(reported);
  });
  it("knows the three words of the engines, and nothing else", () => {
    for (const message of [
      "Failed to fetch dynamically imported module: https://x/a.js",
      "error loading dynamically imported module: https://x/a.js",
      "Importing a module script failed.",
    ]) {
      expect(isModuleLoadError(new TypeError(message))).toBe(true);
    }
    expect(isModuleLoadError(new TypeError("Load failed"))).toBe(false);
    expect(isModuleLoadError("Importing a module script failed.")).toBe(false);
    expect(isModuleLoadError(null)).toBe(false);
  });
});

describe("test_connections when the page counts as leaving", () => {
  it("is leaving from the question to leave until the person stayed, with one timer", () => {
    const page = watched();
    expect(isLeavingPage()).toBe(false);
    page.tell("beforeunload");
    page.tell("beforeunload");
    expect(isLeavingPage()).toBe(true);
    expect(page.timers.size).toBe(1);
    page.timers.values().next().value?.();
    expect(isLeavingPage()).toBe(false);
  });
  it("is leaving for good at pagehide, but not where the browser keeps the page for the way back", () => {
    const page = watched();
    page.tell("pagehide", true);
    expect(isLeavingPage()).toBe(false);
    page.tell("pagehide", false);
    expect(isLeavingPage()).toBe(true);
    page.tell("pageshow");
    expect(isLeavingPage()).toBe(false);
  });
});

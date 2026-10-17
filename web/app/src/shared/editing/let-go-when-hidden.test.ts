import { describe, expect, it } from "vitest";

import { letGoWhenHidden } from "./let-go-when-hidden.ts";

/** A page that can be hidden and shown, with the events a browser fires for it. */
function aPage() {
  const target = new EventTarget();
  const calls: string[] = [];
  const stop = letGoWhenHidden(
    target,
    () => {
      calls.push("let go");
    },
    () => {
      calls.push("come back");
    },
  );
  const shown = (persisted: boolean) => {
    const event = new Event("pageshow");
    Object.defineProperty(event, "persisted", { value: persisted });
    target.dispatchEvent(event);
  };
  return { calls, stop, hidden: () => target.dispatchEvent(new Event("pagehide")), shown };
}

describe("iss_05 a window that is hidden lets its edit lock go", () => {
  it("lets go as the page is hidden, whether it is closed, reloaded or put in the cache", () => {
    const page = aPage();
    page.hidden();
    expect(page.calls).toEqual(["let go"]);
  });

  it("asks the editor to come back only when the page returns from the back and forward cache", () => {
    const page = aPage();
    page.shown(false);
    expect(page.calls).toEqual([]);
    page.hidden();
    page.shown(true);
    expect(page.calls).toEqual(["let go", "come back"]);
  });

  it("listens no more after the way back is taken", () => {
    const page = aPage();
    page.stop();
    page.hidden();
    page.shown(true);
    expect(page.calls).toEqual([]);
  });
});

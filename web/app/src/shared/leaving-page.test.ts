import { describe, expect, it } from "vitest";

import { isLeavingPage, watchLeavingPage } from "./leaving-page.ts";

describe("ux_5 a page that is being left", () => {
  it("is not leaving until the browser says so, and is again no longer when the person stayed", () => {
    const heard = new Map<string, () => void>();
    let restore: (() => void) | undefined;
    watchLeavingPage(
      {
        addEventListener: (type, listener) => {
          heard.set(type, listener);
        },
      },
      (back) => {
        restore = back;
      },
    );
    expect(isLeavingPage()).toBe(false);
    heard.get("beforeunload")?.();
    expect(isLeavingPage()).toBe(true);
    restore?.();
    expect(isLeavingPage()).toBe(false);
    heard.get("pagehide")?.();
    expect(isLeavingPage()).toBe(true);
  });
});

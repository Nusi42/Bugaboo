import { describe, expect, it } from "vitest";

import { isSafeHtml, markSafeHtml, type SafeHtml } from "./safe-html.ts";

/** Words of the tests, not HTML of people. */
const text = "<p>Hi</p>";

describe("sec_13 only the renderers make a SafeHtml", () => {
  it("knows what markSafeHtml made", () => {
    const safe = markSafeHtml(text);
    expect(isSafeHtml(safe)).toBe(true);
    expect(safe.html).toBe(text);
  });

  it("does not know a look-alike, a copy or a plain text", () => {
    const made = markSafeHtml(text);
    const lookAlike = { ...made } as SafeHtml;
    expect(isSafeHtml(lookAlike)).toBe(false);
    expect(isSafeHtml(text)).toBe(false);
    expect(isSafeHtml(null)).toBe(false);
  });
});

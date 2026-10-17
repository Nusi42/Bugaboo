import { describe, expect, it } from "vitest";

import { isUnchanged } from "./value-text.ts";

describe("cf_06 a box that is left sends only what is not saved", () => {
  it("sends nothing when the box was left after its save, before the value was read again", () => {
    // "Acme" was shown; "Acme Ltd" was typed and sent. The page still shows "Acme".
    const kept = { from: "Acme", text: "Acme Ltd" };
    expect(isUnchanged("Acme Ltd", "Acme", kept)).toBe(true);
    // Without the memory of what was sent, the box would send it again.
    expect(isUnchanged("Acme Ltd", "Acme")).toBe(false);
  });

  it("sends a text that differs from what was sent, and follows a value that was read again", () => {
    const kept = { from: "Acme", text: "Acme Ltd" };
    expect(isUnchanged("Acme Ltd Inc", "Acme", kept)).toBe(false);
    // The read came back with what was sent: the shown value is the saved one.
    expect(isUnchanged("Acme Ltd", "Acme Ltd", kept)).toBe(true);
    // Somebody else's value came in: it counts, not what this box sent before.
    expect(isUnchanged("Acme Ltd", "Zed", kept)).toBe(false);
  });
});

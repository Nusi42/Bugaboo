import { describe, expect, it } from "vitest";

import { codePointsOf, cutToCodePoints } from "./code-points.ts";

describe("ai_10 a text is counted and cut by characters, as the server counts them", () => {
  it("counts a picture character once", () => {
    expect(codePointsOf("a\u{1F600}b")).toBe(3);
    expect("a\u{1F600}b".length).toBe(4);
  });

  it("never cuts inside a character", () => {
    expect(cutToCodePoints("a\u{1F600}b", 2)).toBe("a\u{1F600}");
    expect(cutToCodePoints("abc", 5)).toBe("abc");
  });
});

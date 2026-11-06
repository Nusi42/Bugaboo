import { describe, expect, it } from "vitest";

import { choiceNumberOf } from "./choice-number.ts";

describe("time_13 the number a person types for a choice", () => {
  it("is none for an empty box and the digits as typed for digits", () => {
    expect(choiceNumberOf("")).toBeNull();
    expect(choiceNumberOf("  ")).toBeNull();
    expect(choiceNumberOf("4100")).toBe("4100");
    expect(choiceNumberOf(" 0410 ")).toBe("0410");
  });

  it("is refused for anything that is not up to nine digits", () => {
    for (const typed of ["-1", "41.5", "4100a", "1234567890", "K-100", "41 00"]) {
      expect(choiceNumberOf(typed)).toBeUndefined();
    }
  });
});

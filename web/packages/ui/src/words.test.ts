import { describe, expect, it } from "vitest";

import { isGap, wordsAndGaps } from "./words.ts";

describe("ui_words the words of a text and the gaps between them", () => {
  it("keeps every character, in order, and tells a word from a gap", () => {
    const pieces = wordsAndGaps("Added  quarter-report.pdf today");
    expect(pieces.join("")).toBe("Added  quarter-report.pdf today");
    expect(pieces.map((piece) => isGap(piece))).toEqual([false, true, false, true, false]);
  });

  it("does not break a word at a hyphen, and has no empty piece", () => {
    expect(wordsAndGaps(" a-b ")).toEqual([" ", "a-b", " "]);
    expect(wordsAndGaps("")).toEqual([]);
  });
});

import { describe, expect, it } from "vitest";

import { containsEveryWord, isGap, wordsAndGaps, wordsOnly } from "./words.ts";

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

describe("ui_words the words of a text without the gaps", () => {
  it("are the words in order, with no empty one", () => {
    expect(wordsOnly("  Ada  Lovelace\tand\nco ")).toEqual(["Ada", "Lovelace", "and", "co"]);
    expect(wordsOnly("")).toEqual([]);
  });

  it("are all found in a text, in any order and in any case, by containsEveryWord", () => {
    expect(containsEveryWord("Region and time zone", "ZONE  region")).toBe(true);
    expect(containsEveryWord("Region and time zone", "zone dark")).toBe(false);
    expect(containsEveryWord("Region", "   ")).toBe(true);
  });
});

import { describe, expect, it } from "vitest";

import { combine, compareRows, paragraphsOf, wordParts } from "./compare.ts";

describe("iss_17 the two texts of Compare and save, paragraph by paragraph", () => {
  it("splits at empty lines and drops the empty ones", () => {
    expect(paragraphsOf("One\r\n\r\n\r\nTwo\nstill two\n\n  \n")).toEqual([
      "One",
      "Two\nstill two",
    ]);
    expect(paragraphsOf("")).toEqual([]);
  });

  it("puts the paragraphs of both texts in the same place and marks the ones that are the same", () => {
    const rows = compareRows("A\n\nB mine\n\nC", "A\n\nB theirs");
    expect(rows).toEqual([
      { mine: "A", theirs: "A", same: true },
      { mine: "B mine", theirs: "B theirs", same: false },
      { mine: "C", theirs: "", same: false },
    ]);
  });

  it("builds the text from the choices, without the paragraphs that are left empty", () => {
    const rows = compareRows("A\n\nB mine\n\nC", "A\n\nB theirs");
    expect(combine(rows, ["mine", "mine", "mine"])).toBe("A\n\nB mine\n\nC");
    expect(combine(rows, ["mine", "theirs", "theirs"])).toBe("A\n\nB theirs");
    expect(combine(rows, ["theirs", "theirs", "mine"])).toBe("A\n\nB theirs\n\nC");
  });
});

/** The stretches of `text` that the other text does not have. */
const marked = (text: string, other: string) =>
  wordParts(text, other)
    .filter((part) => part.changed)
    .map((part) => part.text);

describe("m3_f54 Show changes marks the words that differ", () => {
  it("marks the words that the other text does not have, and none when the texts are the same", () => {
    expect(marked("The login fails on Monday", "The login fails on Tuesday")).toEqual(["Monday"]);
    expect(marked("The login fails on Tuesday", "The login fails on Monday")).toEqual(["Tuesday"]);
    expect(marked("Same words", "Same words")).toEqual([]);
  });

  it("keeps the text whole: the parts put together are the paragraph", () => {
    const text = "One  two\tthree\nfour";
    expect(
      wordParts(text, "One three five")
        .map((part) => part.text)
        .join(""),
    ).toBe(text);
  });

  it("marks a word that moved or was added, and marks everything against an empty paragraph", () => {
    expect(marked("a b c d", "a c d")).toEqual(["b"]);
    expect(marked("a b c", "c b a").length).toBeGreaterThan(0);
    expect(marked("new words", "")).toEqual(["new words"]);
    expect(wordParts("", "old")).toEqual([]);
  });

  it("does not mark the spaces between two marked words as a word of their own", () => {
    const parts = wordParts("keep one two keep", "keep keep");
    expect(parts.filter((part) => part.changed).map((part) => part.text)).toEqual(["one two"]);
  });
});

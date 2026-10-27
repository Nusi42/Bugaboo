import type { Mistake } from "@bugaboo/wasm-search";
import { testSearchLanguage } from "../../shared/search-language-testing.ts";
import { describe, expect, it } from "vitest";

import { piecesOf } from "./marks.ts";

const language = testSearchLanguage;

describe("qry_03 the words of Advanced search are marked", () => {
  it("marks the field and the value, and leaves the rest of the text plain", () => {
    const text = "status is Open";
    const pieces = piecesOf(text, language.parse(text).search, []);
    expect(pieces.map((piece) => [piece.text, piece.marks])).toEqual([
      ["status", ["field"]],
      [" is ", []],
      ["Open", ["value"]],
    ]);
  });

  it("marks a text between quotes as one value, quotes included", () => {
    const text = 'title contains "a b"';
    const marked = piecesOf(text, language.parse(text).search, []).filter(
      (piece) => piece.marks.length > 0,
    );
    expect(marked.map((piece) => piece.text)).toEqual(["title", '"a b"']);
  });

  it("gives a piece both marks when a value has a mistake, and covers the whole text", () => {
    const text = "status is Open";
    const mistake: Mistake = {
      span: { start: 10, end: 14 },
      key: "search.mistake.unknown-choice",
      args: {},
      suggestions: [],
    };
    const pieces = piecesOf(text, language.parse(text).search, [mistake]);
    expect(pieces.at(-1)?.marks).toEqual(["value", "mistake"]);
    expect(pieces.map((piece) => piece.text).join("")).toBe(text);
  });

  it("marks the letter before a mistake that has no length", () => {
    const mistake: Mistake = {
      span: { start: 3, end: 3 },
      key: "search.mistake.expected-value",
      args: {},
      suggestions: [],
    };
    const pieces = piecesOf("a is", null, [mistake]);
    expect(pieces.filter((piece) => piece.marks.includes("mistake")).map((p) => p.text)).toEqual([
      "i",
    ]);
  });

  it("has a piece for a text with no marks at all, and none for an empty text", () => {
    expect(piecesOf("abc", null, [])).toEqual([{ start: 0, text: "abc", marks: [] }]);
    expect(piecesOf("", null, [])).toEqual([]);
  });
});

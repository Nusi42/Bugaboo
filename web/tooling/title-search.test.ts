import { readFileSync } from "node:fs";

import { moduleLocation } from "@bugaboo/wasm-search";
import { startSearchLanguage } from "@bugaboo/wasm-search/testing";
import { describe, expect, it } from "vitest";

import type { ReadableFields } from "@bugaboo/wasm-search";
import { titleSearchText, titleWord } from "../app/src/shared/title-search.ts";

const language = startSearchLanguage(() => readFileSync(moduleLocation));

/** What every person may read: the title, as a text. */
const readable: ReadableFields = {
  fields: [{ word: titleWord, kind: "text", operators: ["contains"], choices: [] }],
};

describe("cfg_08 the search of Quick search and of the Issue finder for words of a title", () => {
  it("is written by the search language, as a title search that reads back without mistakes", () => {
    const text = titleSearchText(language, "login page");
    expect(text).toBe('title contains "login page*"');
    expect(language.check(String(text), readable).mistakes).toEqual([]);
  });

  it("keeps quotes and backslashes inside the words, so they cannot become the language", () => {
    const words = String.raw`say "hi" \ and sort by`;
    const answer = language.check(String(titleSearchText(language, words)), readable);
    expect(answer.mistakes).toEqual([]);
    expect(answer.search?.sort).toEqual([]);
    expect(answer.search?.filter?.type).toBe("field");
  });

  it("lets the word that is being typed be a start, when the language lets a start be that short", () => {
    expect(titleSearchText(language, "logi")).toBe('title contains "logi*"');
    expect(titleSearchText(language, "fix lo")).toBe('title contains "fix lo*"');
    // One letter is no start; a finished word is a whole word.
    expect(titleSearchText(language, "fix l")).toBe('title contains "fix l"');
    expect(titleSearchText(language, "login ")).toBe('title contains "login"');
    // A letter with an umlaut is one letter.
    expect(titleSearchText(language, "ü")).toBe('title contains "ü"');
    expect(titleSearchText(language, "mü")).toBe('title contains "mü*"');
  });

  it("writes a start that the checker takes, with no mistake", () => {
    for (const typed of ["lo", "mü", "ß", "æ", "fix logi", "x", "e\u0301", "me\u0301"]) {
      const answer = language.check(String(titleSearchText(language, typed)), readable);
      expect(answer.mistakes).toEqual([]);
    }
  });

  it("has nothing to write for an empty box", () => {
    expect(titleSearchText(language, "   ")).toBeUndefined();
  });
});

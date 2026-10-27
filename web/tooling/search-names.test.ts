import { readFileSync } from "node:fs";

import { moduleLocation } from "@bugaboo/wasm-search";
import { startSearchLanguage } from "@bugaboo/wasm-search/testing";
import { describe, expect, it } from "vitest";

import { applyWord, issueSource, nameSource } from "../app/src/shared/apply-suggestion.ts";
import { withSortBy } from "../app/src/shared/sort-by.ts";
import { withFilter } from "../app/src/shared/with-filter.ts";

const language = startSearchLanguage(() => readFileSync(moduleLocation));

describe("qry_03 a suggestion that is taken goes into the text", () => {
  it("replaces what was typed of the word and leaves room for the next", () => {
    expect(applyWord("assi", { start: 0, end: 4 }, "assignedTo")).toEqual({
      text: "assignedTo ",
      cursor: 11,
    });
  });

  it("keeps the rest of the text and does not add a second space", () => {
    expect(applyWord("pri is high", { start: 0, end: 3 }, "priority")).toEqual({
      text: "priority is high",
      cursor: 8,
    });
  });

  it("leaves no space after an opening bracket", () => {
    expect(applyWord("changed in ", { start: 11, end: 11 }, "(")).toEqual({
      text: "changed in (",
      cursor: 12,
    });
  });
});

describe("qry_01 names are written the way the language writes them", () => {
  it("quotes a name that has a space or a quote in it, and only then", () => {
    expect(nameSource(language, "Ana")).toBe("Ana");
    expect(nameSource(language, "Ana Lopez")).toBe('"Ana Lopez"');
    expect(nameSource(language, 'say "hi"')).toBe(String.raw`"say \"hi\""`);
  });

  it("writes an Issue ID as it is", () => {
    expect(issueSource(language, "WEB-142")).toBe("WEB-142");
  });
});

describe("qry_01 sort by is changed through the parser and printer", () => {
  it("replaces the order and keeps the filters", () => {
    expect(withSortBy(language, "status is Open sort by title", "priority", "highest_first")).toBe(
      "status is Open sort by priority, highest first",
    );
    expect(withSortBy(language, "", "title", null)).toBe("sort by title");
  });

  it("returns a text that does not parse as it was", () => {
    expect(withSortBy(language, "status is", "title", null)).toBe("status is");
  });
});

describe("qry_01 a filter is added through the parser and printer", () => {
  it("adds a value by its shown name, quoted where needed", () => {
    expect(withFilter(language, "", "status", "Open")).toBe("status is Open");
    expect(withFilter(language, "assignedTo is me", "status", "In progress")).toBe(
      'assignedTo is me and status is "In progress"',
    );
  });

  it("adds an empty field and keeps the order", () => {
    expect(
      withFilter(language, "status is Open sort by title", "assignedTo", { empty: true }),
    ).toBe("status is Open and assignedTo is empty sort by title");
  });

  it("returns a text that does not parse as it was", () => {
    expect(withFilter(language, "status is", "status", "Open")).toBe("status is");
  });
});

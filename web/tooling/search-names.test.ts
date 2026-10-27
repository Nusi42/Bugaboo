import { readFileSync } from "node:fs";

import { moduleLocation } from "@bugaboo/wasm-search";
import { startSearchLanguage } from "@bugaboo/wasm-search/testing";
import { describe, expect, it } from "vitest";

import { applyWord, issueSource, nameSource } from "../app/src/shared/apply-suggestion.ts";
import { withSortBy } from "../app/src/shared/sort-by.ts";
import { withGroupFilter } from "../app/src/shared/with-filter.ts";

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

describe("qry_01 the language writes and reads values for the browser", () => {
  it("writes a value as it stands in the text, whatever its kind", () => {
    const span = { start: 0, end: 0 };
    expect(language.operandSource({ type: "duration", minutes: 90, span })).toBe(
      "1 hour 30 minutes",
    );
    expect(language.operandSource({ type: "me", span })).toBe("me");
    expect(language.operandSource({ type: "time", word: "this_week", span })).toBe("this week");
    expect(
      language.operandSource({ type: "date", date: { year: 2026, month: 10, day: 25 }, span }),
    ).toBe("2026-10-25");
  });

  it("takes words for an Issue ID only when they are exactly one", () => {
    expect(language.issueIdOf("web-1")).toBe("WEB-1");
    // Words that read as a whole search after `under` are no Issue ID.
    expect(language.issueIdOf("WEB-1 or status is Open")).toBeUndefined();
    expect(language.issueIdOf("printer jams")).toBeUndefined();
    expect(language.issueIdOf("")).toBeUndefined();
  });

  it("reads a length of time only when the text is nothing else", () => {
    expect(language.durationMinutesOf("2 hours")).toBe(120);
    expect(language.durationMinutesOf("45 minutes")).toBe(45);
    expect(language.durationMinutesOf("1 hour 30 minutes")).toBe(90);
    expect(language.durationMinutesOf("2 hours or status is Open")).toBeUndefined();
    expect(language.durationMinutesOf("2")).toBeUndefined();
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
    expect(withGroupFilter(language, "", "status is Open")).toBe("status is Open");
    expect(withGroupFilter(language, "assignedTo is me", 'status is "In progress"')).toBe(
      'assignedTo is me and status is "In progress"',
    );
  });

  it("adds an empty field and keeps the order", () => {
    expect(withGroupFilter(language, "status is Open sort by title", "assignedTo is empty")).toBe(
      "status is Open and assignedTo is empty sort by title",
    );
  });

  it("returns a text that does not parse as it was", () => {
    expect(withGroupFilter(language, "status is", "status is Open")).toBe("status is");
  });
});

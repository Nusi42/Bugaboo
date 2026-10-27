import type { Expr, Field, Search } from "@bugaboo/wasm-search";
import { testSearchLanguage } from "../../shared/search-language-testing.ts";
import { describe, expect, it } from "vitest";

import { dayOf, decimalOf, draftOf, filterOf, testOf, type Draft } from "./filter-draft.ts";
import { filterLabel, operandText, testText } from "./filter-text.ts";
import { nowhere, type FieldFilter } from "../../shared/search-tree.ts";

const language = testSearchLanguage;

function filterOfText(text: string): FieldFilter {
  const { search }: { search: Search | null } = language.parse(text);
  const filter = search?.filter;
  if (filter?.type !== "field") {
    throw new Error(`${text} is no filter on a field`);
  }
  return filter;
}

const status: Field = {
  word: "status",
  kind: "choice",
  operators: ["is", "is_not", "in", "is_empty"],
  choices: ["Open", "Done"],
};

describe("qry_02 the window of a filter keeps what a filter says", () => {
  it.each([
    "status is Open",
    "status is not Done",
    "status in (Open, Done)",
    "status is empty",
    'title contains "crash"',
    "changed in last 7 days",
    "created > 2026-10-25",
    "estimate <= 2.5",
    "timeSpent >= 1 hour 30 minutes",
    'assignedTo member of "Backend"',
    "project within WEB",
  ])("opens %j and comes back to the same filter", (text) => {
    const filter = filterOfText(text);
    const field: Field = { ...status, word: filter.field.word, operators: [] };
    const draft = draftOf(field, filter);
    const back = filterOf(field, draft);
    expect(back).not.toBeNull();
    expect(language.print({ filter: back, sort: [], span: nowhere })).toBe(text);
  });

  it("is not complete until the values it needs are there", () => {
    const empty: Draft = { ...draftOf(status), operator: "is" };
    expect(testOf(empty)).toBeNull();
    expect(testOf({ ...empty, operator: "in" })).toBeNull();
    expect(testOf({ ...empty, operator: "contains", text: "  " })).toBeNull();
    expect(testOf({ ...empty, operator: "in_last", count: 0 })).toBeNull();
    expect(testOf({ ...empty, operator: "is_empty" })).not.toBeNull();
  });

  it("reads a number in its shortest form and refuses text that is none", () => {
    expect(decimalOf("1.50")).toEqual({ value: 15, scale: 1 });
    expect(decimalOf("-3")).toEqual({ value: -3, scale: 0 });
    expect(decimalOf("1,5")).toBeNull();
    expect(decimalOf("")).toBeNull();
  });

  it("reads a day of the calendar only", () => {
    expect(dayOf("2026-10-25")).toMatchObject({ type: "date", date: { month: 10, day: 25 } });
    expect(dayOf("2026-02-30")).toBeNull();
    expect(dayOf("25.10.2026")).toBeNull();
  });
});

/** The filter of a text that is one filter, whatever it is. */
function firstFilter(text: string): Expr {
  const filter = language.parse(text).search?.filter;
  if (filter === null || filter === undefined) {
    throw new Error(`${text} has no filter`);
  }
  return filter;
}

/** The names the server gives, here the word between brackets. */
const nameOf = (word: string): string => `<${word}>`;

describe("qry_02 a filter button says what the filter says", () => {
  it("says the name of the field the server gives and the value", () => {
    const label = filterLabel(language, filterOfText("status is not Done"), nameOf);
    expect(label.name).toBe("<status>");
    expect(label.value).toBe("not Done");
    expect(label.asText).toBe(false);
  });

  it("words a length of time as the issue page does", () => {
    expect(testText(filterOfText("timeSpent >= 1 hour 30 minutes").test)).toContain("1h 30m");
  });

  it("joins the members of an Any of with the words of a list, and shows it as text", () => {
    const label = filterLabel(language, firstFilter("status is Open or status is Done"), nameOf);
    expect(label.asText).toBe(true);
    expect(label.value).toBe("<status>: Open and <status>: Done");
  });

  it("words a list of values and a time word", () => {
    expect(testText(filterOfText("status in (Open, Done)").test)).toBe("Open and Done");
    expect(operandText({ type: "me", span: nowhere })).toBe("me");
  });

  it("shows a filter that has no picker as its own text", () => {
    const label = filterLabel(language, firstFilter("under WEB-1"), nameOf);
    expect(label).toEqual({ name: undefined, value: "under WEB-1", asText: true });
  });
});

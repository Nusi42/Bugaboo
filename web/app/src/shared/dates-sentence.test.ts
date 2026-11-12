import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { dateItems, datesMoveOf, datesSentence, type DatesMove } from "./dates-sentence.ts";
import { testWording } from "./wording-testing.ts";

const lea = { shown: "name", id: "l", name: "Lea" } as const;

/** One side of a date: a day, or none. */
function side(value: string | null) {
  return value === null ? undefined : ({ shown: "typed", kind: "date", value } as const);
}

/** The move the items hold. */
function moveOf(items: readonly Schema<"HistoryEntryItem">[]): DatesMove {
  const found = datesMoveOf(items);
  if (found === undefined) {
    throw new TypeError("The items hold no move of both dates.");
  }
  return found;
}

/** The item of one date changing from one day to another (or from or to none). */
function item(
  field: "start-date" | "due-date",
  before: string | null,
  after: string | null,
): Schema<"HistoryEntryItem"> {
  return {
    event: "e1",
    at: "2026-11-12T10:00:00Z",
    kind: "change",
    by: lea,
    change: { field, before: side(before) ?? null, after: side(after) ?? null },
  };
}

describe("plan_03 both dates moving in one event are one sentence", () => {
  const both = [
    item("start-date", "2026-11-03", "2026-11-10"),
    item("due-date", "2026-11-07", "2026-11-14"),
  ];

  it("takes the days of both dates before and after", () => {
    expect(datesMoveOf(both)).toEqual({
      before: { start: "2026-11-03", due: "2026-11-07" },
      after: { start: "2026-11-10", due: "2026-11-14" },
    });
    expect(dateItems(both)).toHaveLength(2);
  });

  it("says who moved the dates from which days to which", () => {
    const said = datesSentence(testWording, { by: lea }, moveOf(both));
    expect(said).toMatch(/^Lea moved the dates from .*2026.* to .*2026/u);
  });

  it("is no sentence when one date alone changes or one was none", () => {
    expect(datesMoveOf([item("due-date", "2026-11-07", "2026-11-14")])).toBeUndefined();
    expect(
      datesMoveOf([
        item("start-date", null, "2026-11-10"),
        item("due-date", "2026-11-07", "2026-11-14"),
      ]),
    ).toBeUndefined();
  });
});

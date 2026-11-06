import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { planIssueCell, planWorkDayEdit, reasonIsNeeded, type WorkDayEdit } from "./edits.ts";

type Day = Schema<"WorkDayBody">;
type Entry = Schema<"TimeEntryItem">;

const zone = "Europe/Berlin";

/** A day that may run, like today, unless the test says it cannot. */
function plan(edit: Omit<WorkDayEdit, "canRun"> & { readonly canRun?: boolean }) {
  return planWorkDayEdit({ canRun: true, ...edit });
}

function dayOf(periods: Day["periods"], breaks: Day["breaks"] = []): Day {
  return {
    date: "2026-10-05",
    zone,
    periods,
    breaks,
    totals: { worked: 0, breaks: 0 },
    warnings: [],
    runs: false,
    version: 3,
    changed: false,
  };
}

function entry(id: string, minutes: number, version = 1): Entry {
  return {
    id,
    issue: "issue-1",
    readable_id: "WEB-1",
    date: "2026-10-05",
    minutes,
    note: "",
    own_fields: {},
    own_field_names: {},
    version,
  };
}

describe("time_08 the start of a typed day", () => {
  it("starts a day that has no record with a period that runs", () => {
    const planned = plan({
      day: undefined,
      date: "2026-10-05",
      zone,
      field: "start",
      minutes: 485,
    });
    expect(planned).toEqual({
      times: { periods: [{ start: "2026-10-05T06:05:00.000Z", end: null }], breaks: [] },
    });
  });

  it("asks for the start before the end or the breaks of a day without one", () => {
    expect(plan({ day: undefined, date: "2026-10-05", zone, field: "end", minutes: 1020 })).toEqual(
      {
        problem: "time.sheet.start-first",
      },
    );
    expect(
      plan({ day: undefined, date: "2026-10-05", zone, field: "breaks", minutes: 30 }),
    ).toEqual({
      problem: "time.sheet.start-first",
    });
  });
});

describe("time_08 a past day is typed as a whole", () => {
  it("holds a start for a day that cannot run until its end is typed", () => {
    const edit = { day: undefined, date: "2026-10-05", zone };
    expect(plan({ ...edit, field: "start", minutes: 485, canRun: false })).toEqual({ held: 485 });
    expect(plan({ ...edit, field: "end", minutes: 1020, canRun: false, held: 485 })).toEqual({
      times: {
        periods: [{ start: "2026-10-05T06:05:00.000Z", end: "2026-10-05T15:00:00.000Z" }],
        breaks: [],
      },
    });
  });
});

describe("time_08 the end of a typed day", () => {
  it("ends the day at the time typed", () => {
    const day = dayOf([{ start: "2026-10-05T06:05:00.000Z", end: null }]);
    expect(plan({ day, date: day.date, zone, field: "end", minutes: 17 * 60 + 15 })).toEqual({
      times: {
        periods: [{ start: "2026-10-05T06:05:00.000Z", end: "2026-10-05T15:15:00.000Z" }],
        breaks: [],
      },
    });
  });

  it("ends a day on the next morning when the end is not after the start", () => {
    const day = dayOf([{ start: "2026-10-05T20:00:00.000Z", end: null }]);
    const planned = plan({ day, date: day.date, zone, field: "end", minutes: 3 * 60 });
    expect(planned).toEqual({
      times: {
        periods: [{ start: "2026-10-05T20:00:00.000Z", end: "2026-10-06T01:00:00.000Z" }],
        breaks: [],
      },
    });
  });

  it("moves the start of the first period and the end of the last one, and leaves the middle", () => {
    const day = dayOf([
      { start: "2026-10-05T06:05:00.000Z", end: "2026-10-05T11:00:00.000Z" },
      { start: "2026-10-05T12:10:00.000Z", end: "2026-10-05T16:10:00.000Z" },
    ]);
    const start = plan({ day, date: day.date, zone, field: "start", minutes: 8 * 60 });
    expect(start).toMatchObject({
      times: { periods: [{ start: "2026-10-05T06:00:00.000Z" }, {}] },
    });
    const end = plan({ day, date: day.date, zone, field: "end", minutes: 18 * 60 });
    expect(end).toMatchObject({ times: { periods: [{}, { end: "2026-10-05T16:00:00.000Z" }] } });
  });
});

describe("time_08 the breaks of a typed day", () => {
  const day = dayOf([{ start: "2026-10-05T06:00:00.000Z", end: "2026-10-05T14:00:00.000Z" }]);

  it("keeps the times of the breaks as they were typed", () => {
    const planned = plan({
      day,
      date: day.date,
      zone,
      field: "breaks",
      minutes: 0,
      breaks: [
        [10 * 60, 10 * 60 + 15],
        [12 * 60 + 10, 12 * 60 + 40],
      ],
    });
    expect(planned).toEqual({
      times: {
        periods: day.periods,
        breaks: [
          { start: "2026-10-05T08:00:00.000Z", end: "2026-10-05T08:15:00.000Z" },
          { start: "2026-10-05T10:10:00.000Z", end: "2026-10-05T10:40:00.000Z" },
        ],
      },
    });
  });

  it("takes the breaks away when none are typed", () => {
    const planned = plan({ day, date: day.date, zone, field: "breaks", minutes: 0, breaks: [] });
    expect(planned).toEqual({ times: { periods: day.periods, breaks: [] } });
  });
});

describe("time_04 a typed cell of an issue books, changes or takes back time", () => {
  const target = { issue: "WEB-1", date: "2026-10-05" };

  it("books time in an empty cell", () => {
    expect(planIssueCell({ ...target, entries: [], minutes: 90 })).toEqual({
      kind: "add",
      issue: "WEB-1",
      date: "2026-10-05",
      minutes: 90,
    });
  });

  it("changes the one entry of a cell, on the version that was looked at", () => {
    expect(planIssueCell({ ...target, entries: [entry("e1", 60, 4)], minutes: 90 })).toEqual({
      kind: "change",
      entry: "e1",
      version: 4,
      minutes: 90,
    });
  });

  it("changes nothing when the value is the same", () => {
    expect(planIssueCell({ ...target, entries: [entry("e1", 90)], minutes: 90 })).toEqual({
      kind: "none",
    });
  });

  it("deletes the one entry of a cell that is cleared", () => {
    expect(planIssueCell({ ...target, entries: [entry("e1", 60, 2)], minutes: undefined })).toEqual(
      {
        kind: "delete",
        entry: "e1",
        version: 2,
      },
    );
    expect(planIssueCell({ ...target, entries: [], minutes: undefined })).toEqual({ kind: "none" });
  });

  it("leaves a cell of several entries to the details", () => {
    expect(
      planIssueCell({ ...target, entries: [entry("e1", 60), entry("e2", 30)], minutes: 90 }),
    ).toEqual({ kind: "several" });
  });
});

describe("time_10 which change asks for a reason", () => {
  it("asks for a change of one's own past day and of any day of somebody else", () => {
    expect(reasonIsNeeded({ own: true, date: "2026-10-04", today: "2026-10-05" })).toBe(true);
    expect(reasonIsNeeded({ own: true, date: "2026-10-05", today: "2026-10-05" })).toBe(false);
    expect(reasonIsNeeded({ own: true, date: "2026-10-06", today: "2026-10-05" })).toBe(false);
    expect(reasonIsNeeded({ own: false, date: "2026-10-05", today: "2026-10-05" })).toBe(true);
  });
});

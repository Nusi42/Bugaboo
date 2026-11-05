import { describe, expect, it } from "vitest";

import { breakStartOf, endOf, startOf, workDayNow, type WorkDay } from "./work-day-state.ts";

/** A stretch of a day from its start to its end; none while it runs. */
function span([start, end]: readonly [string, string | null]) {
  return { start, end };
}

/** A day of the week with these stretches. */
function day(
  date: string,
  periods: readonly (readonly [string, string | null])[],
  breaks: readonly (readonly [string, string | null])[] = [],
): WorkDay {
  return {
    date,
    zone: "Europe/Berlin",
    periods: periods.map((each) => span(each)),
    breaks: breaks.map((each) => span(each)),
    totals: { worked: 0, breaks: 0 },
    warnings: [],
    runs: [...periods, ...breaks].some(([, end]) => end === null),
    version: 1,
  };
}

const today = "2027-01-14";

describe("time_08 what the work day of today is doing", () => {
  it("is none while no day of today has started", () => {
    expect(workDayNow([], today).kind).toBe("none");
    expect(
      workDayNow([day("2027-01-13", [["2027-01-13T08:00:00Z", "2027-01-13T16:00:00Z"]])], today)
        .kind,
    ).toBe("none");
  });

  it("is working while a period runs and on a break while a break runs", () => {
    const working = day(today, [["2027-01-14T08:00:00Z", null]]);
    const onBreak = day(today, [["2027-01-14T08:00:00Z", null]], [["2027-01-14T12:00:00Z", null]]);
    expect(workDayNow([working], today).kind).toBe("working");
    expect(workDayNow([onBreak], today).kind).toBe("break");
    expect(breakStartOf(onBreak)).toBe("2027-01-14T12:00:00Z");
  });

  it("is ended when nothing runs, with its first start and last end", () => {
    const over = day(today, [["2027-01-14T08:00:00Z", "2027-01-14T16:30:00Z"]]);
    expect(workDayNow([over], today).kind).toBe("ended");
    expect(startOf(over)).toBe("2027-01-14T08:00:00Z");
    expect(endOf(over)).toBe("2027-01-14T16:30:00Z");
  });
});

describe("time_08 a day that was left open", () => {
  it("is the first older day that still runs", () => {
    const open = day("2027-01-12", [["2027-01-12T08:00:00Z", null]]);
    const later = day("2027-01-13", [["2027-01-13T08:00:00Z", null]]);
    expect(workDayNow([open, later], today).leftOpen?.date).toBe("2027-01-12");
  });

  it("is none for a day of today that runs", () => {
    const running = day(today, [["2027-01-14T08:00:00Z", null]]);
    expect(workDayNow([running], today).leftOpen).toBeUndefined();
  });
});

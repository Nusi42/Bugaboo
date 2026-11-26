import { describe, expect, it } from "vitest";

import { inMonth, steppedAnchor, windowOf } from "./span-days.ts";

describe("plan_09_the_days_a_calendar_shows", () => {
  it("shows the weeks that hold a month, from the first day of the week of the person", () => {
    const month = windowOf("2026-11-26", "month", "monday");
    expect(month.first).toBe("2026-10-26");
    expect(month.last).toBe("2026-12-06");
    expect(month.weeks).toHaveLength(6);
    expect(month.weeks.every((week) => week.length === 7)).toBe(true);
    const sunday = windowOf("2026-11-26", "month", "sunday");
    expect(sunday.first).toBe("2026-11-01");
    expect(sunday.last).toBe("2026-12-05");
  });

  it("shows seven days for a week, and never more than six weeks for a month", () => {
    const week = windowOf("2026-11-26", "week", "monday");
    expect([week.first, week.last, week.weeks.length]).toEqual(["2026-11-23", "2026-11-29", 1]);
    expect(windowOf("2027-02-10", "month", "monday").weeks).toHaveLength(4);
    expect(windowOf("2026-08-10", "month", "sunday").weeks.length).toBeLessThanOrEqual(6);
  });

  it("draws the days around a month quieter, and every day of a week alike", () => {
    const month = windowOf("2026-11-26", "month", "monday");
    expect(inMonth(month, "2026-10-28")).toBe(false);
    expect(inMonth(month, "2026-11-02")).toBe(true);
    expect(inMonth(windowOf("2026-11-26", "week", "monday"), "2026-11-23")).toBe(true);
  });

  it("steps a month or a week earlier and later", () => {
    expect(steppedAnchor("2026-11-26", "month", 1)).toBe("2026-12-01");
    expect(steppedAnchor("2026-11-26", "month", -1)).toBe("2026-10-01");
    expect(steppedAnchor("2026-11-26", "week", 1)).toBe("2026-12-03");
    expect(steppedAnchor("2026-11-26", "week", -1)).toBe("2026-11-19");
  });
});

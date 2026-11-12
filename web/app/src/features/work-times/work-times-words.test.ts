import { t } from "@bugaboo/messages";
import { describe, expect, it } from "vitest";

import { formatCalendarDay } from "../../shared/format.ts";
import { deviceRegion, setRegion } from "../../shared/region.ts";
import { awayInOrder, type Away, type UsualDay } from "./work-times.ts";
import { awayDaysText, nextHolidaysText, usualSummary, weekOf } from "./work-times-words.ts";

/** Seven usual days, Monday first: the ticked ones by their place, each with `minutes`. */
function week(ticked: readonly number[], minutes: readonly number[] = []): readonly UsualDay[] {
  return Array.from({ length: 7 }, (_, at) => ({
    works: ticked.includes(at),
    minutes: minutes[at] ?? 480,
  }));
}

/** An away time with the fields the words read. */
function away(id: string, first: string, last: string, half: Partial<Away> = {}): Away {
  return {
    id,
    first,
    last,
    first_afternoon_only: false,
    last_morning_only: false,
    note: null,
    entered_by: null,
    version: 1,
    ...half,
  };
}

describe("plan_11 the words of work times", () => {
  it("orders the week as the Region starts it", () => {
    setRegion({ ...deviceRegion, firstDay: "sunday" });
    expect(weekOf("sunday").map((day) => day.at)).toEqual([6, 0, 1, 2, 3, 4, 5]);
    expect(weekOf("monday").map((day) => day.at)).toEqual([0, 1, 2, 3, 4, 5, 6]);
    expect(weekOf("saturday").map((day) => day.at)).toEqual([5, 6, 0, 1, 2, 3, 4]);
  });

  it("sums up the usual days in runs, a day or a week", () => {
    const monday = weekOf("monday");
    expect(usualSummary(monday, week([0, 1, 2, 3, 4]))).toMatch(/a day$/u);
    expect(usualSummary(monday, week([0, 1, 2, 3], [360, 360, 360, 360]))).toContain("6h");
    // Different hours are summed for the week.
    expect(usualSummary(monday, week([0, 2], [480, 480, 240]))).toContain("12h");
    expect(usualSummary(monday, week([]))).toBe(t("work-times.usual-none"));
  });

  it("writes whole and half days of an away time", () => {
    const day = formatCalendarDay("2027-03-01");
    expect(awayDaysText(away("a", "2027-03-01", "2027-03-01"))).toBe(day);
    expect(
      awayDaysText(away("a", "2027-03-01", "2027-03-01", { first_afternoon_only: true })),
    ).toBe(t("work-times.day-afternoon", { date: day }));
    expect(awayDaysText(away("a", "2027-03-01", "2027-03-05", { last_morning_only: true }))).toBe(
      t("work-times.days", {
        first: day,
        last: t("work-times.day-morning", { date: formatCalendarDay("2027-03-05") }),
      }),
    );
  });

  it("lists the coming away times first, then the past ones latest first, each once", () => {
    const both = away("b", "2026-11-10", "2026-11-14");
    const listed = awayInOrder(
      [both, away("c", "2026-12-01", "2026-12-02")],
      [away("a", "2026-01-05", "2026-01-06"), away("z", "2026-06-01", "2026-06-01"), both],
    );
    expect(listed.coming.map((one) => one.id)).toEqual(["b", "c"]);
    expect(listed.past.map((one) => one.id)).toEqual(["z", "a"]);
  });

  it("says the next holidays, or that none count", () => {
    expect(nextHolidaysText({ next_holidays: [] })).toBe(t("work-times.holidays-none"));
    expect(
      nextHolidaysText({ next_holidays: [{ date: "2026-12-25", holiday: "christmas-day" }] }),
    ).toContain(t("holiday.christmas-day"));
  });
});

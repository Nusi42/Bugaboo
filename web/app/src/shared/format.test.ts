import { describe, expect, it } from "vitest";

import {
  clockPartsOf,
  daysUntil,
  formatAgo,
  formatDate,
  formatDateTime,
  formatSince,
  formatTime,
  isCalendarDay,
  splitUptime,
} from "./format.ts";
import { deviceRegion, type Region } from "./region.ts";

describe("ops_08 formatting for the status page", () => {
  it("splits an uptime into days, hours and minutes", () => {
    expect(splitUptime(0)).toEqual({ days: 0, hours: 0, minutes: 0 });
    expect(splitUptime(59)).toEqual({ days: 0, hours: 0, minutes: 0 });
    expect(splitUptime(3 * 86_400 + 4 * 3600 + 5 * 60 + 6)).toEqual({
      days: 3,
      hours: 4,
      minutes: 5,
    });
  });

  it("counts whole days until a moment, and goes negative after it", () => {
    const now = Date.parse("2026-10-05T12:00:00Z");
    expect(daysUntil("2026-10-15T12:00:00Z", now)).toBe(10);
    expect(daysUntil("2026-10-15T11:59:59Z", now)).toBe(9);
    expect(daysUntil("2026-10-04T12:00:00Z", now)).toBe(-1);
  });

  it("writes a day the way people say it", () => {
    expect(formatDate("2026-10-05T12:00:00Z")).toBe("October\u00A05,\u00A02026");
  });
});

describe("iss_17 how long ago something happened", () => {
  const now = Date.parse("2026-10-05T12:00:00Z");

  it("says minutes, hours and days ago, and now for under a minute", () => {
    expect(formatAgo("2026-10-05T11:58:00Z", now)).toBe("2 minutes ago");
    expect(formatAgo("2026-10-05T11:59:30Z", now)).toBe("this minute");
    expect(formatAgo("2026-10-05T09:00:00Z", now)).toBe("3 hours ago");
    expect(formatAgo("2026-10-03T12:00:00Z", now)).toBe("2 days ago");
    expect(formatAgo("2026-10-05T12:00:05Z", now)).toBe("this minute");
  });

  it("writes a short span for a line like since 2 min", () => {
    expect(formatSince("2026-10-05T11:58:00Z", now)).toBe("2 min");
    expect(formatSince("2026-10-05T09:00:00Z", now)).toBe("3 hr");
  });
});

describe("ux_a4 a date or a time never wraps", () => {
  const regions = ["year-month-day", "day-month-year", "month-day-year", "device"] as const;
  const region = (dateFormat: (typeof regions)[number]): Region => ({
    ...deviceRegion,
    dateFormat,
    timeZone: "UTC",
  });

  it("holds every moment together with spaces and hyphens that do not break", () => {
    for (const dateFormat of regions) {
      const text = [
        formatDateTime("2026-10-18T01:16:00Z", region(dateFormat)),
        formatDate("2026-10-18T01:16:00Z", region(dateFormat)),
        formatTime("2026-10-18T13:16:00Z", region(dateFormat)),
      ];
      for (const each of text) {
        expect(each).not.toMatch(/[ -]/u);
      }
    }
  });

  it("keeps the numbers as they were", () => {
    expect(formatDateTime("2026-10-18T01:16:00Z", region("year-month-day"))).toBe(
      "2026\u201110\u201118,\u00A01:16\u00A0AM",
    );
  });
});

describe("the clock and the calendar of a region", () => {
  const berlin: Region = { ...deviceRegion, timeZone: "Europe/Berlin" };

  it("say the numbers at a moment, on a 24 hour clock, with summer time", () => {
    expect(clockPartsOf(Date.parse("2027-03-01T22:30:05Z"), berlin)).toEqual({
      year: 2027,
      month: 3,
      day: 1,
      hour: 23,
      minute: 30,
      second: 5,
    });
    expect(clockPartsOf(Date.parse("2027-03-30T22:00:00Z"), berlin)).toMatchObject({
      day: 31,
      hour: 0,
    });
  });

  it("know a day on the calendar from text that only looks like one", () => {
    expect(isCalendarDay("2028-02-29")).toBe(true);
    expect(isCalendarDay("2027-02-29")).toBe(false);
    expect(isCalendarDay("2027-2-1")).toBe(false);
  });
});

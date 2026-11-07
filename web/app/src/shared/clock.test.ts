import { describe, expect, it } from "vitest";

import vectors from "../../../../tools/testdata/value-words.json";
import { calendarDayOf } from "./format.ts";
import { deviceRegion, deviceTimeZone } from "./region.ts";
import {
  clockMinutesOf,
  dayOfMonth,
  momentOfClock,
  personToday,
  readClock,
  weekdayName,
  weekDaysOf,
  weekFirstOf,
  zoneOfNewDay,
} from "./clock.ts";

describe("time_08 a time of day typed into a cell of the work day", () => {
  it.each([
    ["8:05", 8 * 60 + 5],
    ["08:05", 8 * 60 + 5],
    ["805", 8 * 60 + 5],
    ["0805", 8 * 60 + 5],
    ["8", 8 * 60],
    ["17", 17 * 60],
    ["17:15", 17 * 60 + 15],
    ["5:15 pm", 17 * 60 + 15],
    ["5:15PM", 17 * 60 + 15],
    ["12:30 am", 30],
    ["12 pm", 12 * 60],
    ["8 am", 8 * 60],
    ["  9:00 ", 9 * 60],
    ["0:00", 0],
  ])("reads %s as %i minutes after midnight", (typed, minutes) => {
    expect(readClock(typed)).toBe(minutes);
  });

  it.each(["", "abc", "25:00", "8:60", "13 pm", "0 pm", "8:5", "1:2:3", "-1", "8,5"])(
    "does not read %j",
    (typed) => {
      expect(readClock(typed)).toBeUndefined();
    },
  );
});

describe("time_08 a time of day on a day in a zone is a moment", () => {
  it("places 8:05 of a day in Berlin in summer and in winter", () => {
    expect(momentOfClock("2026-10-05", 8 * 60 + 5, "Europe/Berlin")).toBe(
      "2026-10-05T06:05:00.000Z",
    );
    expect(momentOfClock("2026-12-07", 8 * 60 + 5, "Europe/Berlin")).toBe(
      "2026-12-07T07:05:00.000Z",
    );
  });

  it("places a time on the day the clocks change", () => {
    // 2027-03-28: the clocks in Berlin jump from 02:00 to 03:00.
    expect(momentOfClock("2027-03-28", 1 * 60, "Europe/Berlin")).toBe("2027-03-28T00:00:00.000Z");
    expect(momentOfClock("2027-03-28", 4 * 60, "Europe/Berlin")).toBe("2027-03-28T02:00:00.000Z");
  });

  it("reads the clock of a moment back", () => {
    expect(clockMinutesOf("2026-10-05T06:05:00.000Z", "Europe/Berlin")).toBe(8 * 60 + 5);
    expect(clockMinutesOf("2026-10-05T23:30:00.000Z", "America/New_York")).toBe(19 * 60 + 30);
  });
});

describe("time_04 the week of a day", () => {
  it("starts on the first day of the person", () => {
    expect(weekFirstOf("2026-10-07", "monday")).toBe("2026-10-05");
    expect(weekFirstOf("2026-10-07", "sunday")).toBe("2026-10-04");
    expect(weekFirstOf("2026-10-07", "saturday")).toBe("2026-10-03");
    expect(weekFirstOf("2026-10-05", "monday")).toBe("2026-10-05");
    expect(weekFirstOf("2026-10-11", "monday")).toBe("2026-10-05");
  });

  it("counts a device that chose nothing from Monday, like the server", () => {
    expect(weekFirstOf("2026-10-07", "device")).toBe("2026-10-05");
  });

  it("lists seven days across a month and a year", () => {
    expect(weekDaysOf("2026-12-28")).toEqual([
      "2026-12-28",
      "2026-12-29",
      "2026-12-30",
      "2026-12-31",
      "2027-01-01",
      "2027-01-02",
      "2027-01-03",
    ]);
  });
});

describe("time_04 the names of the days of the head", () => {
  it("writes the weekday and the number of a day", () => {
    expect(weekdayName("2026-10-05", "short")).toBe("Mon");
    expect(weekdayName("2026-10-08", "long")).toBe("Thursday");
    expect(dayOfMonth("2026-10-05")).toBe(5);
  });
});

describe("time_08 the one today of a person", () => {
  const lateEvening = Date.parse("2027-01-14T23:30:00Z");
  const berlin = { ...deviceRegion, timeZone: "Europe/Berlin" };

  it("is the date in the zone the person chose", () => {
    expect(personToday(berlin, lateEvening)).toBe("2027-01-15");
    expect(personToday({ ...berlin, timeZone: "America/Los_Angeles" }, lateEvening)).toBe(
      "2027-01-14",
    );
  });

  it("is the date of the device for a person whose Region is the device or UTC, as the server starts their day", () => {
    const onDevice = calendarDayOf(lateEvening, {
      ...deviceRegion,
      dateFormat: "year-month-day",
      timeZone: deviceTimeZone(),
    });
    expect(personToday({ ...deviceRegion, timeZone: "device" }, lateEvening)).toBe(onDevice);
    expect(personToday({ ...deviceRegion, timeZone: "UTC" }, lateEvening)).toBe(onDevice);
  });
});

const UNKNOWN_ZONE = "Nowhere/Land";

/** The cases of the server's choice that a browser can follow: it knows its own zone and the zones of the list. */
type NewDayZone = Readonly<{ chosen: string | null; device: string | null; expected: string }>;

const newDayZones = vectors.newDayZones
  .filter((each: NewDayZone) => each.chosen !== UNKNOWN_ZONE && each.device !== UNKNOWN_ZONE)
  .map((each: NewDayZone) => ({
    chosen: each.chosen ?? "device",
    device: each.device ?? "UTC",
    expected: each.expected,
  }));

describe("time_08 a new work day starts in the zone the server decides", () => {
  it.each(newDayZones)(
    "starts in $expected for $chosen on $device",
    (zones: Readonly<{ chosen: string; device: string; expected: string }>) => {
      expect(zoneOfNewDay(zones.chosen, zones.device)).toBe(zones.expected);
    },
  );
});

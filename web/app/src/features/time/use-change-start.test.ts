import { describe, expect, it } from "vitest";

import { deviceRegion } from "../../shared/region.ts";
import { startBoxes, startOf } from "./use-change-start.ts";

describe("time_01 saving the start of a timer unchanged sends the same moment", () => {
  const started = "2027-01-14T23:30:00Z";

  it.each(["UTC", "Europe/Berlin", "America/New_York", "Asia/Tokyo"])(
    "in the zone %s",
    (timeZone) => {
      const region = { ...deviceRegion, timeZone, dateFormat: "year-month-day" } as const;
      const { day, time } = startBoxes(Date.parse(started), region);
      expect(startOf(day, time, region)).toBe(new Date(started).toISOString());
    },
  );

  it("takes the day of the start on the clock of the region, not today", () => {
    const region = { ...deviceRegion, timeZone: "UTC", dateFormat: "year-month-day" } as const;
    expect(startBoxes(Date.parse(started), region)).toEqual({ day: "2027-01-14", time: "23:30" });
  });
});

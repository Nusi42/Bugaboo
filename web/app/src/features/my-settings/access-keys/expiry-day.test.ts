import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Region } from "../../../shared/region.ts";
import { deviceRegion } from "../../../shared/region.ts";
import { bodyOf } from "./draft.ts";
import { earliestDay, endOfDay, latestDay, presetDay } from "./expiry-day.ts";

const rules = { default_days: 90, longest_days: 365 } as const;

const kiritimati: Region = { ...deviceRegion, timeZone: "Pacific/Kiritimati" };
const pagoPago: Region = { ...deviceRegion, timeZone: "Pacific/Pago_Pago" };
const berlin: Region = { ...deviceRegion, timeZone: "Europe/Berlin" };

const filledIn = {
  name: "Nightly report",
  choice: "see-only" as const,
  single: false,
  picked: [],
  projects: [],
  password: "a password",
};

/** What the clock says in a zone at an instant, as `2027-01-23 23:59:59`; empty for no instant. */
function wallClock(instant: string | null, zone: string): string {
  return instant === null
    ? ""
    : new Intl.DateTimeFormat("sv-SE", {
        timeZone: zone,
        dateStyle: "short",
        timeStyle: "medium",
      }).format(new Date(instant));
}

/** The clock of the region at the end of a day, as `2027-01-23 23:59:59`. */
function endClock(day: string, region: Region): string {
  return wallClock(endOfDay(day, region), region.timeZone);
}

const SUMMER_TIME_DAYS = ["2027-03-27", "2027-03-28", "2027-03-29", "2027-10-31"] as const;
const NOT_DAYS = ["", "23.01.2027", "27-01-23", "2027-13-01", "2027-02-30"] as const;

describe("the end of the day a key runs out", () => {
  it("is the last second of the day on the clock of the region, whatever the day is in UTC", () => {
    expect(endClock("2027-01-23", kiritimati)).toBe("2027-01-23 23:59:59");
    expect(endClock("2027-01-23", pagoPago)).toBe("2027-01-23 23:59:59");
  });

  it.each(SUMMER_TIME_DAYS)("holds on %s, across the change of summer time", (day) => {
    expect(endClock(day, berlin)).toBe(`${day} 23:59:59`);
  });

  it.each(NOT_DAYS)("is no instant for the text %j, which is not a day", (text) => {
    expect(endOfDay(text, berlin)).toBeNull();
  });

  it("is what is sent for the day the person picked, and none while the person picked none", () => {
    const picked = bodyOf({ ...filledIn, day: "2027-01-23" }, berlin).expires_at;
    expect(wallClock(picked, "Europe/Berlin")).toBe("2027-01-23 23:59:59");
    expect(bodyOf({ ...filledIn, day: null }).expires_at).toBeNull();
  });
});

describe("the days a key may run out on", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    // 23:30 UTC on New Year's Eve: the 1st of January in Kiritimati, the 31st of December in Pago Pago.
    vi.setSystemTime(new Date("2026-12-31T23:30:00Z"));
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it("are counted from today on the calendar of the region, once", () => {
    expect(earliestDay(kiritimati)).toBe("2027-01-02");
    expect(earliestDay(pagoPago)).toBe("2027-01-01");
    expect(presetDay(rules, kiritimati)).toBe("2027-04-01");
    expect(presetDay(rules, pagoPago)).toBe("2027-03-31");
    expect(latestDay(rules, kiritimati)).toBe("2027-12-31");
  });

  it("never put the default later than the latest day, even when it is the longest time", () => {
    const same = { default_days: 365, longest_days: 365 };
    expect(presetDay(same, pagoPago)).toBe(latestDay(same, pagoPago));
  });
});

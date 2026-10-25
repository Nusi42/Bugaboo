import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { Region } from "../../../shared/region.ts";
import { deviceRegion } from "../../../shared/region.ts";
import { bodyOf } from "./draft.ts";
import {
  type Days,
  defaultDaysOf,
  earliestDay,
  endOfKey,
  keyEndOf,
  latestDay,
  presetDay,
} from "./expiry-day.ts";

const rules: Days = { default_days: 90, longest_days: 365 };

/** Rules that cap no day of these tests. */
const farAway = { default_days: 1, longest_days: 3650 } as const;

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

/** The clock of the region at the end of a key picked for a day, as `2027-01-23 23:59:59`. */
function endClock(day: string, region: Region, rule: Days = farAway): string {
  return wallClock(endOfKey(day, rule, region), region.timeZone);
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
    expect(endOfKey(text, farAway, berlin)).toBeNull();
  });

  it("is what is sent for the day the person picked, and none while the person picked none", () => {
    const picked = endOfKey("2027-01-23", { default_days: 90, longest_days: 3650 }, berlin);
    expect(wallClock(picked, "Europe/Berlin")).toBe("2027-01-23 23:59:59");
    expect(bodyOf(filledIn, picked).expires_at).toBe(picked);
    expect(bodyOf(filledIn, null).expires_at).toBeNull();
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
    expect(latestDay(rules, kiritimati)).toBe("2028-01-01");
  });

  it.each([
    [30, "2027-01-31", "2027-01-30"],
    [365, "2028-01-01", "2027-12-31"],
  ])(
    "put the default on the latest day, the day the server's moment falls on, when both are %i days",
    (days, kiritimatiDay, pagoDay) => {
      const same = { default_days: days, longest_days: days };
      expect(presetDay(same, kiritimati)).toBe(kiritimatiDay);
      expect(latestDay(same, kiritimati)).toBe(kiritimatiDay);
      expect(presetDay(same, pagoPago)).toBe(pagoDay);
      expect(latestDay(same, pagoPago)).toBe(pagoDay);
    },
  );

  it("cut a default that is longer than the longest time to it", () => {
    const rule = { default_days: 90, longest_days: 30 };
    expect(presetDay(rule, pagoPago)).toBe("2027-01-30");
  });
});

describe("the days a key runs for by default", () => {
  it("are the default, cut to the longest time and to at least one day", () => {
    expect(defaultDaysOf({ default_days: 90, longest_days: 365 })).toBe(90);
    expect(defaultDaysOf({ default_days: 90, longest_days: 30 })).toBe(30);
    expect(defaultDaysOf({ default_days: 0, longest_days: 30 })).toBe(1);
  });
});

describe("the days around the change of summer time (Europe/Berlin)", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("are the days of the moments 90 and 365 days on, not a day short", () => {
    // 23:30 on the 1st of March 2027 in Berlin (CET); 90 days on it is 00:30 CEST on the 31st of May.
    vi.useFakeTimers({ now: new Date("2027-03-01T22:30:00Z") });
    expect(presetDay(rules, berlin)).toBe("2027-05-31");
    expect(latestDay(rules, berlin)).toBe("2028-02-29");
  });

  it("keep the box, the latest day and the list on the same day when the clocks go back", () => {
    // 00:30 CEST on the 31st of October 2027, a day of 25 hours: 24 hours on is 23:30 on that day.
    vi.useFakeTimers({ now: new Date("2027-10-30T22:30:00Z") });
    const one = { default_days: 1, longest_days: 1 };
    expect(earliestDay(berlin)).toBe("2027-10-31");
    expect(latestDay(one, berlin)).toBe("2027-10-31");
    expect(presetDay(one, berlin)).toBe("2027-10-31");
    expect(endClock("2027-10-31", berlin, one)).toBe("2027-10-31 23:30:00");
  });

  it("end a picked day at its last second, or at the longest time when that comes first", () => {
    // 23:30 CET on the 27th of March 2027; the clocks go forward in the night to the 28th.
    vi.useFakeTimers({ now: new Date("2027-03-27T22:30:00Z") });
    const one = { default_days: 1, longest_days: 1 };
    expect(latestDay(one, berlin)).toBe("2027-03-29");
    expect(endOfKey("2027-03-28", one, berlin)).toBe("2027-03-28T21:59:59.000Z");
    expect(endOfKey("2027-03-29", one, berlin)).toBe("2027-03-28T22:30:00.000Z");
  });

  it("take a day that is no day for a problem of the form, never for the default", () => {
    vi.useFakeTimers({ now: new Date("2027-03-27T22:30:00Z") });
    expect(keyEndOf(null, rules, berlin)).toEqual({ valid: true, expiresAt: null });
    expect(keyEndOf("2027-02-30", rules, berlin)).toEqual({ valid: false, expiresAt: null });
    expect(keyEndOf("2027-04-02", rules, berlin)).toEqual({
      valid: true,
      expiresAt: "2027-04-02T21:59:59.000Z",
    });
  });

  it("send no moment for text that is no day", () => {
    vi.useFakeTimers({ now: new Date("2027-03-27T22:30:00Z") });
    expect(endOfKey("2027-02-30", rules, berlin)).toBeNull();
    expect(endOfKey("", rules, berlin)).toBeNull();
  });
});

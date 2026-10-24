import { describe, expect, it } from "vitest";

import { bodyOf } from "./draft.ts";
import { dayText, endOfDay, localDay, presetDay } from "./expiry-day.ts";

const rules = { default_days: 90, longest_days: 365 } as const;

const filledIn = {
  name: "Nightly report",
  choice: "see-only" as const,
  single: false,
  picked: [],
  projects: [],
  password: "a password",
};

describe("the day a key runs out", () => {
  it("ends on the day of the box on the person's own clock, whatever the day is in UTC", () => {
    const end = new Date(endOfDay("2027-01-23"));
    expect([end.getFullYear(), end.getMonth(), end.getDate()]).toEqual([2027, 0, 23]);
    expect([end.getHours(), end.getMinutes(), end.getSeconds()]).toEqual([23, 59, 59]);
  });

  it("is the day that is filled in when the person picked none", () => {
    expect(presetDay(rules)).toBe(dayText(localDay(rules.default_days)));
    const body = bodyOf({ ...filledIn, day: presetDay(rules) });
    expect(dayText(new Date(body.expires_at).getTime())).toBe(presetDay(rules));
  });

  it("is never later than the latest day, even when the default is as long as the longest time", () => {
    expect(presetDay({ default_days: 365, longest_days: 365 })).toBe(dayText(localDay(364)));
  });
});

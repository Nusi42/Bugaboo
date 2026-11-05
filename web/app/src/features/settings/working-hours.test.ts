import { describe, expect, it } from "vitest";

import type { Setting } from "../../shared/my-settings.ts";
import { whoSeesWorkingHours } from "./who-sees.ts";
import { controlKindOf, unitWords } from "./texts.ts";

const hr = { id: "role-hr", name: "HR", permissions: ["see-working-hours-of-others"] } as const;
const member = { id: "role-member", name: "Team member", permissions: ["see-projects"] } as const;

const group = { kind: "group", id: "g1", name: "Payroll" } as const;
const person = { kind: "person", id: "p1", name: "Hana Hart" } as const;

describe("time_12 who holds see working hours of others", () => {
  it("lists each group or person once that holds a role with the permission", () => {
    const rows = [
      { who: group, role: { id: hr.id } },
      { who: person, role: { id: member.id } },
      { who: group, role: { id: hr.id } },
      { who: person, role: null },
      { who: person, role: { id: hr.id } },
    ];
    expect(whoSeesWorkingHours([hr, member], rows)).toEqual([group, person]);
  });

  it("lists nobody while no role holds the permission", () => {
    expect(whoSeesWorkingHours([member], [{ who: group, role: { id: member.id } }])).toEqual([]);
  });
});

/** A number of minutes, as the registry sends it. */
const minutes: Setting = {
  key: "working-hours.day-minutes-max",
  name_key: "setting.working-hours.day-minutes-max.name",
  help_key: "setting.working-hours.day-minutes-max.help",
  kind: "whole-number",
  format: "free",
  choices: [],
  levels: ["organization"],
  lockable: false,
  default: 600,
  value: 600,
  source: { kind: "default" },
  own: false,
  locked: false,
  bounds: { least: 60, most: 1440 },
  unit: "minutes",
};

describe("time_09 the numbers of the rules are lengths", () => {
  it("draws a number of minutes as a length and any other number as a number", () => {
    expect(controlKindOf(minutes)).toBe("length");
    expect(controlKindOf({ ...minutes, unit: null })).toBe("typed");
  });
});

describe("time_10 a number of years has its unit beside the box", () => {
  it("says what each unit draws in one place", () => {
    const years: Setting = { ...minutes, key: "working-hours.keep-years", unit: "years", value: 2 };
    expect(unitWords(years, 1)).toBe("year");
    expect(unitWords(years, 2)).toBe("years");
    expect(unitWords(minutes, 2)).toBeUndefined();
    expect(unitWords({ ...minutes, unit: null }, 2)).toBeUndefined();
    expect(controlKindOf(years)).toBe("typed");
  });
});

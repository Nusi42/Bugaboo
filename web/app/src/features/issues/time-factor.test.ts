import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { factorForPeople, type TimeFactor } from "./time-factor.ts";

const lea: Schema<"PersonRef"> = { shown: "name", id: "lea", name: "Lea" };
const ben: Schema<"PersonRef"> = { shown: "name", id: "ben", name: "Ben" };

/** Lea's own factor of ×1.5, as the server answers it to a reader who may change it. */
const leas: TimeFactor = {
  issue: "WEB-1",
  version: 3,
  percent: 150,
  own_percent: null,
  own_estimate: true,
  person: lea,
  set_by: null,
  may_change: true,
};

const reader = { id: "ada", mayChangeField: true };

describe("plan_14 the time factor the row shows after a change of the people", () => {
  it("keeps the answer while the person on the bar is the same", () => {
    expect(factorForPeople(leas, lea, reader)).toBe(leas);
  });

  it("goes back to ×1, set by nobody, when another person is put first", () => {
    expect(factorForPeople(leas, ben, reader)).toEqual({
      ...leas,
      percent: 100,
      own_estimate: false,
      person: ben,
      may_change: true,
    });
  });

  it("is the reader's own when the reader is put first, also without the field level", () => {
    const shown = factorForPeople(leas, ben, { id: "ben", mayChangeField: false });
    expect([shown.own_percent, shown.may_change]).toEqual([100, true]);
  });

  it("has nobody and nothing to change when nobody is assigned", () => {
    const shown = factorForPeople(leas, undefined, reader);
    expect([shown.person, shown.may_change]).toEqual([null, false]);
  });

  it("waits for the answer when the first person is shown only by a role", () => {
    expect(factorForPeople(leas, { shown: "role", role: "Team member" }, reader)).toBe(leas);
  });
});

import { describe, expect, it, vi } from "vitest";

import { formatNumber, keepingHyphens } from "../../../shared/format.ts";
import type { PlanWarning } from "../timeline-data.ts";
import { warningSentence } from "./warning-words.ts";

// The days of a stretch come from the module of the dates of the timeline, which brings the saving of
// a change with it; these sentences name no stretch.
vi.mock("../dates.ts", () => ({ daysText: () => "" }));

const lea = { shown: "name", id: "l", name: "Lea" } as const;

/** A warning of a kind, with nothing in it but what a test gives. */
function warning(more: Partial<PlanWarning>): PlanWarning {
  return {
    issue: "i",
    kind: "delay",
    sentence: "plan-warning.delay-goal",
    days: { first: "2027-03-01", last: "2027-03-05" },
    person: null,
    away_days: null,
    other: null,
    late_days: null,
    needs_days: null,
    has_days: null,
    worst_day: null,
    planned_minutes: null,
    can_minutes: null,
    ways_out: [],
    move_after: null,
    due_needed: null,
    accepted: null,
    ...more,
  };
}

const goal = { key: "WEB-280", title: "Mobile app 2.0", project: "Mobile app" };
const login = { id: "s", readable_id: "WEB-281", title: "Login screen" };

describe("the sentences of plan warnings (UX §4.7, PLAN-13)", () => {
  it("plan_13_a_goal_late_by_an_away_week_names_the_person_and_the_titles", () => {
    const away = warning({
      sentence: "plan-warning.delay-goal-away",
      person: lea,
      away_days: 5,
      other: login,
      late_days: 3,
    });
    expect(warningSentence(away, goal)).toBe(
      "Lea's away week moves Login screen past the due date of Mobile app 2.0.",
    );
    expect(warningSentence({ ...away, away_days: 2 }, goal)).toBe(
      "Lea's 2 away days move Login screen past the due date of Mobile app 2.0.",
    );
  });

  it("plan_13_a_late_goal_is_named_by_its_title_and_by_its_id_where_the_title_is_hidden", () => {
    const late = warning({ other: login, late_days: 3 });
    expect(warningSentence(late, goal)).toBe(
      "Login screen now ends 3 days after the due date of Mobile app 2.0.",
    );
    const hidden = warningSentence(
      { ...late, other: { ...login, title: null } },
      { ...goal, title: null },
    );
    expect(hidden).toBe(
      `${keepingHyphens("WEB-281")} now ends 3 days after the due date of ${keepingHyphens("WEB-280")}.`,
    );
  });

  it("plan_13_too_much_at_once_writes_its_hours_as_hours", () => {
    const much = warning({
      kind: "too-much-at-once",
      sentence: "plan-warning.too-much-at-once",
      person: lea,
      worst_day: "2027-03-02",
      planned_minutes: 540,
      // 6 hours a day at 60 %.
      can_minutes: 216,
    });
    const said = warningSentence(much, goal);
    expect(said).toContain("Lea has 9 hours planned on");
    expect(said).toContain(`in Mobile app; the share there is ${formatNumber(3.6)} hours.`);
  });
});

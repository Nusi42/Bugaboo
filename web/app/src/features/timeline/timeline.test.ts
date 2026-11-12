import { describe, expect, it } from "vitest";

import { keyMoveOf } from "./bar-keys.ts";
import { moved, shownWhileMoving } from "./moves.ts";
import { firstAround, placeOf, scaleUnits, spanFrom, stepped, weekends } from "./scale.ts";

/** A key of a bar, with or without *Alt* and *Shift*. */
function key(name: string, altKey = false, shiftKey = false) {
  return keyMoveOf({ key: name, altKey, shiftKey });
}

describe("the scale of the timeline (UX §4.7)", () => {
  it("plan_04_starts_around_today_at_each_zoom_and_steps_a_quarter_of_its_span", () => {
    expect(firstAround("2027-03-03", "day", "monday")).toBe("2027-02-24");
    expect(firstAround("2027-03-03", "week", "monday")).toBe("2027-02-15");
    expect(firstAround("2027-03-03", "month", "monday")).toBe("2027-02-01");
    expect(firstAround("2027-05-20", "quarter", "monday")).toBe("2027-04-01");
    const span = spanFrom("2027-03-01", "week");
    expect(span.days).toBe(182);
    expect(stepped(span.first, "week", 1)).toBe("2027-04-16");
    expect(placeOf(span, "2027-03-08")).toBe(7);
  });

  it("plan_04_cuts_its_units_at_the_ends_of_the_span", () => {
    const span = spanFrom("2027-03-01", "quarter");
    const units = scaleUnits(span, "quarter", "monday");
    expect(units[0]).toMatchObject({ key: "2027-03-01", start: 0, length: 31 });
    expect(units[1]).toMatchObject({ key: "2027-04-01", start: 31, length: 91 });
    expect(units.reduce((all, unit) => all + unit.length, 0)).toBe(span.days);
  });

  it("plan_04_shades_weekends_only_where_a_day_is_wide_enough", () => {
    const span = spanFrom("2027-03-01", "day");
    expect(weekends(span, "day")[0]).toEqual({ start: 5, length: 2 });
    expect(weekends(span, "month")).toEqual([]);
  });
});

describe("a drag and its keys (PLAN-03)", () => {
  const bar = { first: "2027-03-03", last: "2027-03-05" };

  it("plan_03_moves_the_kept_dates_and_an_end_never_passes_the_other", () => {
    const kept = { start: "2027-03-03", end: "2027-03-05" };
    expect(moved(kept, bar, "both", 7)).toEqual({ start: "2027-03-10", end: "2027-03-12" });
    expect(moved({ start: null, end: "2027-03-05" }, bar, "both", -1)).toEqual({
      start: null,
      end: "2027-03-04",
    });
    expect(moved(kept, bar, "end", -5)).toEqual({ start: "2027-03-03", end: "2027-03-03" });
    expect(moved(kept, bar, "start", 1)).toEqual({ start: "2027-03-04", end: "2027-03-05" });
    expect(shownWhileMoving(bar, "start", 9)).toEqual({ first: "2027-03-05", last: "2027-03-05" });
  });

  it("plan_03_every_drag_has_a_key", () => {
    expect(key("ArrowRight")).toEqual({ move: "both", days: 1 });
    expect(key("ArrowLeft", false, true)).toEqual({ move: "both", days: -7 });
    expect(key("ArrowRight", true)).toEqual({ move: "end", days: 1 });
    expect(key("ArrowLeft", true, true)).toEqual({ move: "start", days: -1 });
    expect(key("Enter")).toBeUndefined();
  });
});

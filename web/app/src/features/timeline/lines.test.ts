import { describe, expect, it, vi } from "vitest";

import type { GroupKey } from "./lines.ts";
import type { TimelineGoal, TimelineRow } from "./timeline-data.ts";

// The names of the groups come from the list, which reads a stored choice when it is loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(() => null),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { linesOf } = await import("./lines.ts");

/** A row of the timeline: an issue, without a bar, in a group. */
function rowOf(id: string, group: GroupKey): TimelineRow {
  return { bar: null, copy: false, group, issue: { id }, may_move: true };
}

/** The lines of the rows, each told by the type of its group and its count, or by its issue. */
function shown(rows: readonly TimelineRow[]): string[] {
  const goals = new Map<string, TimelineGoal>();
  return linesOf(rows, [], (id) => goals.get(id)).map((line) =>
    line.kind === "group" ? `[${line.key.type}] ${String(line.count)}` : line.issue,
  );
}

describe("the rows grouped by goal (UX §4.7)", () => {
  const goal: GroupKey = { type: "issue", value: "goal" };
  const none: GroupKey = { type: "empty" };

  it("plan_04_a_goal_heads_its_issues_and_is_not_a_row_of_no_goal_again", () => {
    expect(
      shown([
        rowOf("first", goal),
        rowOf("second", goal),
        rowOf("goal", none),
        rowOf("alone", none),
      ]),
    ).toEqual(["[issue] 2", "first", "second", "[empty] 1", "alone"]);
  });

  it("plan_04_a_goal_that_is_the_only_issue_without_a_goal_leaves_no_empty_group", () => {
    expect(shown([rowOf("first", goal), rowOf("goal", none)])).toEqual(["[issue] 1", "first"]);
  });
});

describe("the group without a value (UX §4.7)", () => {
  const release: GroupKey = { type: "release", value: "r" };
  const later: GroupKey = { type: "release", value: "s" };
  const none: GroupKey = { type: "empty" };

  it("plan_04_none_comes_last_over_two_pages", () => {
    // The first page starts with an issue of no release; the second page adds another release.
    const first = [rowOf("loose", none), rowOf("shipped", release)];
    const second = [rowOf("loose too", none), rowOf("next", later)];
    expect(shown([...first, ...second])).toEqual([
      "[release] 1",
      "shipped",
      "[release] 1",
      "next",
      "[empty] 2",
      "loose",
      "loose too",
    ]);
  });
});

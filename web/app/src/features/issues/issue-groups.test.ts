import { describe, expect, it } from "vitest";

import { groupByStatus } from "./issue-groups.ts";
import type { IssueLine } from "./issues.ts";

/** The status each group has in the starting workflow. */
const statusOf = { "not-started": "open", "in-progress": "in-progress", done: "done" } as const;

/** A line of a list with only what grouping reads; the rest is the same for all. */
function line(key: string, group: IssueLine["group"]): IssueLine {
  return {
    id: key,
    key,
    title: key,
    status: statusOf[group],
    group,
    priority: "normal",
    issueType: { id: "type", name: "Bug", icon: "bug", retired: false },
    assignedTo: [],
    changedAt: undefined,
  };
}

describe("iss_01 the issues of a list are grouped by status group", () => {
  it("puts what is in progress first, then what waits, then what is done", () => {
    const groups = groupByStatus([
      line("WEB-1", "done"),
      line("WEB-2", "not-started"),
      line("WEB-3", "in-progress"),
    ]);
    expect(groups.map((each) => each.group)).toEqual(["in-progress", "not-started", "done"]);
  });

  it("keeps the order of the list inside a group and leaves out a group without issues", () => {
    const groups = groupByStatus([
      line("WEB-5", "not-started"),
      line("WEB-4", "done"),
      line("WEB-3", "not-started"),
    ]);
    expect(groups.map((each) => each.group)).toEqual(["not-started", "done"]);
    expect(groups[0]?.lines.map((each) => each.key)).toEqual(["WEB-5", "WEB-3"]);
  });

  it("has no groups for no issues", () => {
    expect(groupByStatus([])).toEqual([]);
  });
});

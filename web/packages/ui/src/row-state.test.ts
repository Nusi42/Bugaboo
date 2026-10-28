import { describe, expect, it } from "vitest";

import { partOf } from "./parts.ts";
import { rowStates } from "./row-state.ts";
import { part, partWords } from "./tokens.ts";

describe("ux_r4 a row carries the state of its status group and of its priority", () => {
  it("every status group is tinted, only critical and high are edged", () => {
    expect(rowStates({ group: "not-started", priority: "normal" })).toEqual(["group-not-started"]);
    expect(rowStates({ group: "in-progress", priority: "low" })).toEqual(["group-in-progress"]);
    expect(rowStates({ group: "done", priority: "high" })).toEqual(["group-done", "priority-high"]);
    expect(rowStates({ group: "in-progress", priority: "critical" })).toEqual([
      "group-in-progress",
      "priority-critical",
    ]);
  });

  it("every row that can carry them has the words in the theme file", () => {
    const words = (["not-started", "in-progress", "done"] as const).flatMap((group) =>
      rowStates({ group, priority: "critical" }),
    );
    const rows = [partWords[part.listRow], partWords[part.issueRow], partWords[part.timelineBar]];
    for (const row of rows) {
      for (const word of words) {
        expect(row.state).toContain(word);
      }
    }
    const attributes = partOf(part.listRow, {
      states: rowStates({ group: "done", priority: "high" }),
    });
    expect(attributes["data-state"]).toBe("group-done priority-high");
  });
});

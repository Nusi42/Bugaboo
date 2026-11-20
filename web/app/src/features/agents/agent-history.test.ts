import { describe, expect, it } from "vitest";

import { deedWords, type AgentDeed } from "./agent-history.ts";

/** An issue as the history names it. */
function issue(readable: string): AgentDeed["issues"][number] {
  return {
    id: `id-${readable}`,
    readable_id: readable,
    project: { id: "p1", short_name: "WEB", name: "Website" },
    issue_type: { id: "t1", name: "Task", icon: "square-check", retired: false },
  };
}

const at = "2026-11-20T09:00:00Z";

describe("the history of an agent", () => {
  it("names the issues it handed over and the agent it handed them to", () => {
    const words = deedWords({
      id: "h1",
      kind: "hand-over",
      at,
      to: { id: "a2", name: "Helper" },
      issues: [issue("WEB-1"), issue("WEB-2")],
    });
    expect(words.issues).toEqual(["WEB-1", "WEB-2"]);
    expect(words.sentence).toContain("WEB-1");
    expect(words.sentence).toContain("WEB-2");
    expect(words.sentence).toContain("Helper");
  });

  it("says another agent when it may not name it, and leaves out issues the reader may not see", () => {
    const words = deedWords({ id: "h2", kind: "hand-over", at, issues: [] });
    expect(words.issues).toEqual([]);
    expect(words.sentence).not.toContain("{");
    expect(words.sentence).toContain("another agent");
  });

  it("says how many changes an undo put back, with or without its issue", () => {
    const seen = deedWords({ id: "e1", kind: "undo", at, issues: [issue("WEB-3")], went_back: 2 });
    expect(seen.sentence).toContain("WEB-3");
    expect(seen.sentence).toContain("2");
    const unseen = deedWords({ id: "e2", kind: "undo", at, issues: [], went_back: 1 });
    expect(unseen.issues).toEqual([]);
    expect(unseen.sentence).toContain("1");
  });
});

import { describe, expect, it } from "vitest";

import { draftOf, limitsOf } from "./limits-draft.ts";

const most = { changes_a_minute: 100, issues_at_once: 20, attempts_on_one_issue: 10 };
const now = { changes_a_minute: 60, issues_at_once: 5, attempts_on_one_issue: 5 };

describe("ai_37 the limits of an agent", () => {
  it("start as the limits that stand", () => {
    expect(limitsOf(draftOf(now), most)).toEqual(now);
  });

  it("are whole numbers from 1 to the most the organization allows", () => {
    const draft = draftOf(now);
    expect(limitsOf({ ...draft, changes: "100" }, most)?.changes_a_minute).toBe(100);
    expect(limitsOf({ ...draft, changes: "101" }, most)).toBeUndefined();
    expect(limitsOf({ ...draft, issues: "0" }, most)).toBeUndefined();
    expect(limitsOf({ ...draft, attempts: "many" }, most)).toBeUndefined();
    expect(limitsOf({ ...draft, attempts: "" }, most)).toBeUndefined();
  });
});

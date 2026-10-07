import type { Schema } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import { isUsed, roleUsedText } from "./used-by-text.ts";

/** What a role is used for, for a test: only what the test names. */
function usedBy(change: Partial<Schema<"RoleUsedByBody">>): Schema<"RoleUsedByBody"> {
  return { groups: 0, people: 0, projects: 0, whole_organization: false, ...change };
}

describe("what a role is used for", () => {
  it("says nobody has it, and never counts zero people", () => {
    const text = roleUsedText(usedBy({ whole_organization: true }));
    expect(text).toContain("Nobody has it");
    expect(text).not.toMatch(/\b0\b/u);
  });

  it("says one person, no groups, and the other way round", () => {
    expect(roleUsedText(usedBy({ projects: 1, people: 1 }))).toContain(
      "1 person and no groups have it",
    );
    expect(roleUsedText(usedBy({ projects: 3, groups: 2 }))).toContain(
      "no people and 2 groups have it",
    );
  });

  it("says a role that nobody has in a place the viewer sees is nothing", () => {
    expect(isUsed(usedBy({}))).toBe(false);
    expect(roleUsedText(usedBy({}))).toContain("Nobody has this role");
  });
});

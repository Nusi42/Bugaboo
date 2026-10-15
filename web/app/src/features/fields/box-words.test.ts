import { describe, expect, it } from "vitest";

import { boxWords, labelOf } from "./box-words.ts";
import type { OwnField } from "./fields.ts";

function field(rules: OwnField["rules"]): OwnField {
  return {
    applies_to: "issues",
    choices: [],
    help: "",
    id: "customer",
    kind: "text",
    look: "plain-text",
    may_manage: true,
    name: "Customer",
    never_to_assistant: false,
    retired: false,
    rules,
    scope: { whole_organization: true, projects: [], issue_types: [] },
    version: 1,
  };
}

describe("cf_03 a field that must be filled in says so in its name", () => {
  it("adds (required) to the name of a required field only", () => {
    expect(labelOf(field({ required: true }))).toBe("Customer (required)");
    expect(labelOf(field({}))).toBe("Customer");
  });

  it("carries the name, the help and what is wrong into the box", () => {
    const words = boxWords(
      { ...field({ required: true }), help: "Who asked" },
      "Must be filled in",
    );
    expect(words).toEqual({
      label: "Customer (required)",
      description: "Who asked",
      errorMessage: "Must be filled in",
    });
  });
});

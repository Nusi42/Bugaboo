// A builder of the fields for the tests of the own fields.
import type { OwnField } from "./fields.ts";

/** A field for the tests; `more` replaces what it is given. */
export function field(more: Partial<OwnField>): OwnField {
  return {
    applies_to: "issues",
    choices: [
      { id: "minor", name: "Minor", color: "cat3", retired: false },
      { id: "major", name: "Major", color: "cat4", icon: "flag", retired: true },
    ],
    help: "",
    id: "severity",
    kind: "pick-one",
    look: "badge",
    may_manage: true,
    name: "Severity",
    never_to_assistant: false,
    retired: false,
    rules: {},
    scope: { whole_organization: true, projects: [], issue_types: [] },
    version: 1,
    ...more,
  };
}

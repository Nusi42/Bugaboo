import { describe, expect, it } from "vitest";

import { boxWords, labelOf } from "./box-words.ts";
import type { OwnField } from "./fields.ts";
import { field as fieldOf } from "./fields-testing.ts";

/** The field of these tests: a text field *Customer* with the rules of the test. */
function field(rules: OwnField["rules"]): OwnField {
  return fieldOf({
    id: "customer",
    kind: "text",
    look: "plain-text",
    name: "Customer",
    choices: [],
    rules,
  });
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

  it("hides the name only in a row of the Details, which shows it already (M3-F187)", () => {
    expect(boxWords(field({}), undefined, true)).toEqual({ label: "Customer", hideLabel: true });
    expect(boxWords(field({}))).toEqual({ label: "Customer" });
  });
});

import { describe, expect, it } from "vitest";

import { actionName, applyName } from "./action-names.ts";
import { manyChangeKind } from "./many-changes.ts";

describe("iss_09 the button that applies a change says what it does and for how many issues", () => {
  it("names the change and the number", () => {
    expect(applyName(manyChangeKind.change, 12)).toBe("Change 12 issues");
    expect(applyName(manyChangeKind.label, 1)).toBe("Change 1 issue");
  });

  it("says Delete for the deletion, so nobody deletes by pressing a button that says Change", () => {
    expect(applyName(manyChangeKind.delete, 12)).toBe("Delete 12 issues");
    expect(actionName(manyChangeKind.delete)).toBe("Delete");
  });
});

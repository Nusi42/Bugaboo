import { describe, expect, it } from "vitest";

import { actionName, applyName } from "./action-names.ts";
import { bulkKind } from "./bulk.ts";

describe("iss_09 the button that applies a change says what it does and for how many issues", () => {
  it("names the change and the number", () => {
    expect(applyName(bulkKind.change, 12)).toBe("Change 12 issues");
    expect(applyName(bulkKind.label, 1)).toBe("Change 1 issue");
  });

  it("says Delete for the deletion, so nobody deletes by pressing a button that says Change", () => {
    expect(applyName(bulkKind.delete, 12)).toBe("Delete 12 issues");
    expect(actionName(bulkKind.delete)).toBe("Delete");
  });
});

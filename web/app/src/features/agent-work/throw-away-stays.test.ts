import { describe, expect, it } from "vitest";

import { staysField } from "./throw-away-stays.ts";

describe("ai_35 what stays when the work is thrown away", () => {
  const skipped = [
    { issue: { id: "I1" }, field: "priority" },
    { issue: null, field: "title" },
  ];

  it("names a field of an issue the preview skips, and no other", () => {
    expect(staysField(skipped, "I1", "priority")).toBe(true);
    expect(staysField(skipped, "I1", "title")).toBe(false);
    expect(staysField(skipped, "I2", "priority")).toBe(false);
  });
});

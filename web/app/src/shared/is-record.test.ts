import { describe, expect, it } from "vitest";

import { isRecord } from "./is-record.ts";

describe("isRecord", () => {
  it("is true for a plain object only", () => {
    expect(isRecord({ a: 1 })).toBe(true);
    expect(isRecord({})).toBe(true);
    expect(isRecord(null)).toBe(false);
    expect(isRecord([])).toBe(false);
    expect(isRecord("text")).toBe(false);
    expect(isRecord(Number.NaN)).toBe(false);
  });
});

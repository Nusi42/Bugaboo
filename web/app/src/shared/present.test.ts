import { describe, expect, it } from "vitest";

import { holdsAll } from "./present.ts";

interface Item {
  readonly id?: string | null;
  readonly name?: string | null;
  readonly note?: string | null;
}

describe("cf_01 an item of a list is the whole thing when it holds the members the whole thing has", () => {
  const names = ["id", "name"] as const;

  it("holds them all", () => {
    const item: Item = { id: "1", name: "Severity" };
    expect(holdsAll(item, names)).toBe(true);
  });

  it("does not hold one that is missing or null, and does not need the others", () => {
    const missing: Item = { id: "1" };
    const empty: Item = { id: "1", name: null };
    expect(holdsAll(missing, names)).toBe(false);
    expect(holdsAll(empty, names)).toBe(false);
    expect(holdsAll({ id: "1", name: "x" }, names)).toBe(true);
  });
});

import { describe, expect, it } from "vitest";

import { lockCanBeRetaken, retakingLockHelps } from "./long-text-model.ts";

describe("adr_0019 taking the lock again helps when the time ran out or somebody took over", () => {
  it("helps for those two reasons only", () => {
    expect(lockCanBeRetaken("ran-out")).toBe(true);
    expect(lockCanBeRetaken("taken-over")).toBe(true);
    expect(lockCanBeRetaken("version-changed")).toBe(false);
  });

  it("reads the same from the code of a refusal", () => {
    expect(retakingLockHelps("edit.lock-lost")).toBe(true);
    expect(retakingLockHelps("edit.taken-over")).toBe(true);
    expect(retakingLockHelps("edit.version-changed")).toBe(false);
    expect(retakingLockHelps("access.forbidden")).toBe(false);
    expect(retakingLockHelps()).toBe(false);
  });
});

import { describe, expect, it } from "vitest";

import { MAX_TICKED, tickAllOffer } from "./tick-all.ts";

describe("cfg_02 tick all results ticks up to five thousand", () => {
  it("offers nothing when the rows shown are all the results", () => {
    expect(tickAllOffer({ shown: 13, total: 13 })).toEqual({ kind: "none" });
  });

  it("offers all results when more match, up to the limit", () => {
    expect(tickAllOffer({ shown: 50, total: 302 })).toEqual({ kind: "all", count: 302 });
    expect(tickAllOffer({ shown: 50, total: MAX_TICKED })).toEqual({
      kind: "all",
      count: MAX_TICKED,
    });
  });

  it("asks to narrow the search above the limit", () => {
    expect(MAX_TICKED).toBe(5000);
    expect(tickAllOffer({ shown: 50, total: MAX_TICKED + 1 })).toEqual({
      kind: "narrow",
      limit: MAX_TICKED,
    });
  });
});

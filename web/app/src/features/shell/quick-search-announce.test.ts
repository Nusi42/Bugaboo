import { describe, expect, it } from "vitest";

import { announcement } from "./quick-search-announce.ts";
import type { QuickFound } from "./use-quick-results.ts";

/** What quick search found: so many results in one group, and whether it is still waiting or a source failed. */
function found(count: number, rest: Partial<QuickFound> = {}): QuickFound {
  const results = Array.from({ length: count }, (_, index) => ({
    id: String(index),
    title: `Result ${String(index)}`,
    path: "/",
  }));
  return {
    groups: count === 0 ? [] : [{ id: "g", heading: "G", results }],
    failed: false,
    waiting: false,
    quiet: true,
    ...rest,
  };
}

describe("cfg_08 what a screen reader says about the results", () => {
  it("says how many results there are", () => {
    expect(announcement(found(1))).toEqual(["1 result"]);
    expect(announcement(found(3))).toEqual(["3 results"]);
  });

  it("says Nothing found, in the words of the list, when there are none", () => {
    expect(announcement(found(0))).toEqual(["Nothing found. Try another word."]);
  });

  it("says that some results could not be loaded", () => {
    expect(announcement(found(2, { failed: true }))).toEqual([
      "2 results",
      "Some results could not be loaded.",
    ]);
  });

  it("says nothing while the server has yet to answer", () => {
    expect(announcement(found(0, { waiting: true }))).toEqual([]);
  });
});

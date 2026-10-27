import { describe, expect, it } from "vitest";

import { focusIsFree, momentCame, type Moments } from "./use-fresh-list.ts";

describe("cfg_08 when the list is made new", () => {
  it("does when the typing pause ends", () => {
    expect(momentCame({ waiting: true, quiet: false }, { waiting: true, quiet: true })).toBe(true);
  });

  it("does when the answer lands", () => {
    expect(momentCame({ waiting: true, quiet: true }, { waiting: false, quiet: true })).toBe(true);
  });

  it.each<[string, Moments, Moments]>([
    ["typing goes on", { waiting: true, quiet: false }, { waiting: true, quiet: false }],
    [
      "nothing changed after the answer",
      { waiting: false, quiet: true },
      { waiting: false, quiet: true },
    ],
    ["a new word is typed", { waiting: false, quiet: true }, { waiting: true, quiet: false }],
  ])("does not when %s", (_, was, now) => {
    expect(momentCame(was, now)).toBe(false);
  });
});

describe("cfg_08 whether the focus goes to the first result", () => {
  it("goes when the box points at nothing", () => {
    expect(focusIsFree(null, "first")).toBe(true);
  });

  it("goes when the box points at the first row as drawn", () => {
    expect(focusIsFree("first", "first")).toBe(true);
  });

  it("stays when the box points at another row, such as the old first row that a group landing on top pushed down", () => {
    expect(focusIsFree("old-first", "new-first")).toBe(false);
    expect(focusIsFree("third", "first")).toBe(false);
  });
});

import { describe, expect, it } from "vitest";

import { combine, compareRows, paragraphsOf } from "./compare.ts";

describe("iss_17 the two texts of Compare and save, paragraph by paragraph", () => {
  it("splits at empty lines and drops the empty ones", () => {
    expect(paragraphsOf("One\r\n\r\n\r\nTwo\nstill two\n\n  \n")).toEqual([
      "One",
      "Two\nstill two",
    ]);
    expect(paragraphsOf("")).toEqual([]);
  });

  it("puts the paragraphs of both texts in the same place and marks the ones that are the same", () => {
    const rows = compareRows("A\n\nB mine\n\nC", "A\n\nB theirs");
    expect(rows).toEqual([
      { mine: "A", theirs: "A", same: true },
      { mine: "B mine", theirs: "B theirs", same: false },
      { mine: "C", theirs: "", same: false },
    ]);
  });

  it("builds the text from the choices, without the paragraphs that are left empty", () => {
    const rows = compareRows("A\n\nB mine\n\nC", "A\n\nB theirs");
    expect(combine(rows, ["mine", "mine", "mine"])).toBe("A\n\nB mine\n\nC");
    expect(combine(rows, ["mine", "theirs", "theirs"])).toBe("A\n\nB theirs");
    expect(combine(rows, ["theirs", "theirs", "mine"])).toBe("A\n\nB theirs\n\nC");
  });
});

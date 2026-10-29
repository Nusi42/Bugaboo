import { describe, expect, it, vi } from "vitest";

import { columnsThatFit, leastWidth } from "./fit-columns.ts";

vi.mock(import("@bugaboo/ui"), async (importOriginal) => ({
  ...(await importOriginal()),
  // Ten pixels for each token, so the sums do not depend on the theme.
  pixelsOf: () => 10,
}));

const words = ["status", "id", "title", "priority", "assignedTo", "changed"];
const fixed = ["status", "id", "title", "priority"];

describe("ux_4_2 the columns that fit", () => {
  it("keeps every column where the room is not measured yet or is enough", () => {
    expect(columnsThatFit(words, undefined, true)).toEqual(words);
    expect(columnsThatFit(words, leastWidth(words, true), true)).toEqual(words);
  });

  it("drops the columns of fields from the right, one by one, until the rest fits", () => {
    const five = leastWidth(words.slice(0, 5), true);
    expect(columnsThatFit(words, five, true)).toEqual(words.slice(0, 5));
    expect(columnsThatFit(words, five - 1, true)).toEqual(fixed);
  });

  it("never drops the status, the ID, the title and the priority", () => {
    expect(columnsThatFit(words, 1, true)).toEqual(fixed);
  });
});

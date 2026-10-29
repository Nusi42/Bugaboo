import { describe, expect, it } from "vitest";

import { columnsThatFit } from "./fit-columns.ts";

// There is no page in these tests, so every token is 0 px: only the widths given count.
const words = ["status", "id", "title", "priority", "assignedTo", "changed"];
const fixed = ["status", "id", "title", "priority"];
const hundred = Object.fromEntries(words.map((word) => [word, 100]));
const input = { ticks: false, widths: hundred, keyChars: 8 } as const;

describe("ux_4_2 the columns that fit", () => {
  it("keeps every column where the room is not measured yet or is enough", () => {
    expect(columnsThatFit(words, { ...input, room: undefined })).toEqual(words);
    expect(columnsThatFit(words, { ...input, room: 600 })).toEqual(words);
  });

  it("drops the columns of fields from the right, one by one, until the rest fits", () => {
    expect(columnsThatFit(words, { ...input, room: 599 })).toEqual(words.slice(0, 5));
    expect(columnsThatFit(words, { ...input, room: 499 })).toEqual(fixed);
  });

  it("never drops the status, the ID, the title and the priority", () => {
    expect(columnsThatFit(words, { ...input, room: 1 })).toEqual(fixed);
  });

  it("counts the width a person gave a column: a wide Assigned to takes its column and the one after it away", () => {
    const wide = { ...hundred, assignedTo: 400 };
    expect(columnsThatFit(words, { ...input, widths: wide, room: 900 })).toEqual(words);
    expect(columnsThatFit(words, { ...input, widths: wide, room: 899 })).toEqual(words.slice(0, 5));
    expect(columnsThatFit(words, { ...input, widths: wide, room: 700 })).toEqual(fixed);
  });
});

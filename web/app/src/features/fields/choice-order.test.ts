import { describe, expect, it } from "vitest";

import { orderAfterDrop, orderAfterMove, orderable } from "./choice-order.ts";
import type { Choice } from "./fields.ts";

function choice(id: string, number?: string): Choice {
  return {
    id,
    name: id,
    shown_as: id,
    search_words: [id],
    color: "cat1",
    retired: false,
    number: number ?? null,
  };
}

const choices = [choice("a", "4100"), choice("b", "5200"), choice("x"), choice("y"), choice("z")];

describe("time_13 the choices that are put in order by hand are those without a number", () => {
  it("leaves the numbered ones out", () => {
    expect(orderable(choices).map((each) => each.id)).toEqual(["x", "y", "z"]);
  });

  it("moves among the unnumbered ones and keeps the numbered ones at the head", () => {
    // Move down the first unnumbered choice: it is the second of them now, a change that shows.
    expect(orderAfterMove(choices, 0, 1)).toEqual(["a", "b", "y", "x", "z"]);
    expect(orderAfterMove(choices, 2, 1)).toEqual(["a", "b", "x", "z", "y"]);
  });

  it("drops before an unnumbered choice or at the end of them, never before a numbered one", () => {
    expect(orderAfterDrop(choices, "z", 0)).toEqual(["a", "b", "z", "x", "y"]);
    expect(orderAfterDrop(choices, "x", 3)).toEqual(["a", "b", "y", "z", "x"]);
    expect(orderAfterDrop(choices, "a", 0)).toBeUndefined();
  });
});

import { describe, expect, it } from "vitest";

import { inLayoutOrder, type RowGroup } from "./details-order.ts";

const own = "0190f4c2-7a3e-7c1d-9b52-3d6e8f1a2b4c";

/** The groups of the Details as they come in, each drawn as a letter. */
function groups(): readonly RowGroup<string>[] {
  return [
    { places: ["assigned-to"], node: "assigned" },
    { places: ["priority"], node: "priority" },
    { places: ["reported-by"], node: "reported" },
    { places: "own-fields", node: "own" },
    { places: ["tags"], node: "tags" },
    { places: "end", node: "followers" },
  ];
}

describe("cfg_01 the Details follow the layout", () => {
  it("keeps the order the rows came in before the layout is read", () => {
    expect(inLayoutOrder([], groups())).toEqual([
      "assigned",
      "priority",
      "reported",
      "own",
      "tags",
      "followers",
    ]);
  });

  it("puts each group where the first of its fields stands in the layout", () => {
    expect(
      inLayoutOrder(
        ["description", "tags", "priority", own, "assigned-to", "reported-by"],
        groups(),
      ),
    ).toEqual(["tags", "priority", "own", "assigned", "reported", "followers"]);
  });

  it("puts a group the layout does not place after the ones it places", () => {
    expect(inLayoutOrder(["priority"], groups())).toEqual([
      "priority",
      "assigned",
      "reported",
      "own",
      "tags",
      "followers",
    ]);
  });
});

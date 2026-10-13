import { describe, expect, it } from "vitest";

import { relationChoices } from "./relations.ts";

const blocks = {
  id: "k1",
  forward: "blocks",
  backward: "is blocked by",
  retired: false,
  version: 1,
};
const related = {
  id: "k2",
  forward: "related to",
  backward: "related to",
  retired: false,
  version: 1,
};

describe("iss_04 the words offered when a relation is added", () => {
  it("offers both ends of a kind, and a kind that reads alike once", () => {
    expect(
      relationChoices([blocks, related]).map((choice) => [choice.name, choice.direction]),
    ).toEqual([
      ["blocks", "forward"],
      ["is blocked by", "backward"],
      ["related to", "forward"],
    ]);
  });

  it("does not offer a retired kind", () => {
    expect(relationChoices([{ ...blocks, retired: true }, related])).toHaveLength(1);
  });
});

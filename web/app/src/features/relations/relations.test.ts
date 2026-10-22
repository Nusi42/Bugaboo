import { describe, expect, it } from "vitest";

import { exactName } from "../../shared/same-name.ts";
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

/** Stands in for the rule of the server (capitals and the spaces inside a name make no difference). */
function standIn(name: string): string {
  return name.split(" ").filter(Boolean).join(" ").toLowerCase();
}

describe("iss_04 the words offered when a relation is added", () => {
  it("offers both ends of a kind, and a kind that reads alike once", () => {
    expect(
      relationChoices([blocks, related], exactName).map((choice) => [
        choice.name,
        choice.direction,
      ]),
    ).toEqual([
      ["blocks", "forward"],
      ["is blocked by", "backward"],
      ["related to", "forward"],
    ]);
  });

  it("does not offer a retired kind", () => {
    expect(relationChoices([{ ...blocks, retired: true }, related], exactName)).toHaveLength(1);
  });

  it("takes the words that the key of the name makes one as reading alike", () => {
    const shouting = { ...related, backward: "RELATED  TO" };
    expect(relationChoices([shouting], standIn)).toHaveLength(1);
    expect(relationChoices([shouting], exactName)).toHaveLength(2);
  });
});

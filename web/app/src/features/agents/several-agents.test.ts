import { describe, expect, it } from "vitest";

import { allKeysText, namesOf, type CreatedAgent } from "./several-agents.ts";

function created(name: string, secret: string): CreatedAgent {
  return {
    agent: { name, keys: [], maximums: null },
    key: {
      secret,
      key: {
        id: "k",
        name,
        prefix: "p",
        choice: "see-and-change",
        permissions: [],
        projects: [],
        created_at: "2026-11-19T00:00:00Z",
        expires_at: "2027-02-17T00:00:00Z",
        expired: false,
      },
    },
  };
}

describe("several agents", () => {
  it("ai_30 reads one name a line and leaves out the empty lines", () => {
    expect(namesOf(" Planner \n\nBuilder\n  \nChecker")).toEqual(["Planner", "Builder", "Checker"]);
  });

  it("ai_30 copies every key with the name of its agent, in the order of the list", () => {
    expect(allKeysText([created("Planner", "bb_one"), created("Builder", "bb_two")])).toBe(
      "Planner: bb_one\nBuilder: bb_two",
    );
  });
});

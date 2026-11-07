import { describe, expect, it } from "vitest";

import { whoOf } from "./change-sentence.ts";

const ada = { shown: "name", id: "p1", name: "Ada Brandt" } as const;
const role = { shown: "role", role: "Team member" } as const;

describe("who did it", () => {
  it("ai_23 names the app the person worked through", () => {
    expect(whoOf({ by: ada, app: "Claude Code" })).toBe("Ada Brandt via Claude Code");
    expect(whoOf({ by: ada })).toBe("Ada Brandt");
    expect(whoOf({ by: ada, app: null })).toBe("Ada Brandt");
  });

  it("ai_23 names an agent with its person, only for a reader who is told the person", () => {
    const fixer = { id: "a1", name: "Fixer" };
    expect(whoOf({ by: ada, agent: fixer })).toBe("Fixer, agent of Ada Brandt");
    // A reader who is not told the person is not told the agent either.
    expect(whoOf({ by: role, agent: fixer })).not.toContain("Fixer");
  });

  it("ai_23 says Bugaboo when nobody is named", () => {
    expect(whoOf({})).toBe("Bugaboo");
  });
});

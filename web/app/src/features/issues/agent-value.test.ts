import { describe, expect, it } from "vitest";

import { agentValueOf, isAgentWord } from "./agent-value.ts";

describe("ai_33 the agent columns of a list row", () => {
  it("names the two columns of the agent and no other word", () => {
    expect(isAgentWord("agent")).toBe(true);
    expect(isAgentWord("agent state")).toBe(true);
    expect(isAgentWord("assigned to")).toBe(false);
  });

  it("shows the agent and where its work is, and nothing where no agent works", () => {
    const agent = { name: "Fixer", state: "agent-working" } as const;
    expect(agentValueOf("agent", agent)).toBe("Fixer");
    expect(agentValueOf("agent state", agent)).toBe("Agent working");
    expect(agentValueOf("agent")).toBeUndefined();
  });
});

import { describe, expect, it } from "vitest";

import { listedAgents, mayAct, runsOutSoon } from "./agents.ts";

const day = 86_400_000;

describe("agents", () => {
  it("lists only agents that came with all they need", () => {
    const listed = listedAgents([
      { id: "a1", name: "Fixer", state: "active", version: 3 },
      { id: "a2", name: "Half" },
    ]);
    expect(listed.map((agent) => agent.id)).toEqual(["a1"]);
    expect(listed[0]?.offered).toBe(false);
  });

  it("lets a stopped agent rest for good", () => {
    expect(mayAct({ state: "active" })).toBe(true);
    expect(mayAct({ state: "paused" })).toBe(true);
    expect(mayAct({ state: "stopped" })).toBe(false);
  });

  it("calls a rule about to run out three days before its end, and only while it works", () => {
    const now = Date.parse("2026-11-01T00:00:00Z");
    const ends = (days: number) => new Date(now + days * day).toISOString();
    expect(runsOutSoon({ state: "open", ends_at: ends(2) }, now)).toBe(true);
    expect(runsOutSoon({ state: "open", ends_at: ends(3) }, now)).toBe(true);
    expect(runsOutSoon({ state: "open", ends_at: ends(10) }, now)).toBe(false);
    expect(runsOutSoon({ state: "ended", ends_at: ends(1) }, now)).toBe(false);
  });
});

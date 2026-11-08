import { describe, expect, it } from "vitest";

import { agentPill, agentsInOrder, pausedSentence, waitingInOrder } from "./agent-state.ts";
import { listedAgents, mayAct, runsOutSoon, type ListedAgent, type MyWorkItem } from "./agents.ts";

const day = 86_400_000;

/** An agent of the tests: active, with the limits of the start. */
function testAgent(name: string, state: ListedAgent["state"], id = name): ListedAgent {
  return {
    id,
    name,
    state,
    pausedFor: undefined,
    version: 1,
    offered: false,
    yours: true,
    person: undefined,
    changesAMinute: 60,
    issuesAtOnce: 5,
    attemptsOnOneIssue: 3,
  };
}

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

  it("takes the limit an agent was paused for from the answer", () => {
    const listed = listedAgents([
      { id: "a1", name: "Fixer", state: "paused", version: 3, paused_for: "issues-at-once" },
      { id: "a2", name: "Sorter", state: "paused", version: 1 },
    ]);
    expect(listed.map((one) => one.pausedFor)).toEqual(["issues-at-once", undefined]);
  });
});

describe("ai_37 the order of agents", () => {
  it("puts the agents that need action or work first, then the paused, then the stopped, by name in each", () => {
    const all = [
      testAgent("Sorter", "stopped"),
      testAgent("fixer", "paused"),
      testAgent("Zed", "active"),
      testAgent("Alpha", "stopped"),
      testAgent("Beta", "active"),
      testAgent("Ada", "paused"),
    ];
    expect(agentsInOrder(all).map((one) => one.name)).toEqual([
      "Beta",
      "Zed",
      "Ada",
      "fixer",
      "Alpha",
      "Sorter",
    ]);
  });

  it("puts a paused agent with something that waits for its person among the first", () => {
    const all = [
      testAgent("Zed", "active"),
      testAgent("Planner", "paused", "p1"),
      testAgent("Ada", "paused"),
    ];
    expect(agentsInOrder(all, ["p1"]).map((one) => one.name)).toEqual(["Planner", "Zed", "Ada"]);
  });

  it("gives the same order whatever order the agents came in", () => {
    const all = [testAgent("B", "active"), testAgent("A", "stopped"), testAgent("C", "paused")];
    expect(agentsInOrder(all.toReversed())).toEqual(agentsInOrder(all));
  });
});

describe("ai_37 the state of an agent", () => {
  it("says the state with the words and the looks of the pill of agent work", () => {
    expect(agentPill("active")).toEqual({ variant: "agent-working", text: "Agent working" });
    expect(agentPill("paused")).toEqual({ variant: "paused", text: "Paused" });
    expect(agentPill("stopped")).toEqual({ variant: "agent-stopped", text: "Stopped" });
  });

  it("says which limit paused an agent, with its number", () => {
    const paused = { ...testAgent("Fixer", "paused"), pausedFor: "changes-a-minute" as const };
    expect(pausedSentence(paused)).toBe("Fixer paused: 60 changes in a minute");
    expect(pausedSentence({ ...paused, pausedFor: "attempts-on-one-issue" })).toBe(
      "Fixer paused: 3 attempts on one issue",
    );
    expect(pausedSentence(testAgent("Sorter", "paused"))).toBeUndefined();
    expect(pausedSentence({ ...paused, state: "active" })).toBeUndefined();
  });
});

/** A piece of work of the tests that waits for `waits` on the issue `key`. */
function waiting(key: string, waits: NonNullable<MyWorkItem["waits_for"]> | null) {
  return { issue: { readable_id: key }, waits_for: waits } as const;
}

describe("ai_37 the order of Waiting for you", () => {
  it("lists questions, then plans, then results, by issue, and leaves out what waits for nothing", () => {
    const work = [
      waiting("WEB-10", "review"),
      waiting("WEB-3", "plan"),
      waiting("WEB-12", "question"),
      waiting("WEB-2", null),
      waiting("WEB-9", "question"),
    ];
    expect(waitingInOrder(work).map((item) => item.issue.readable_id)).toEqual([
      "WEB-9",
      "WEB-12",
      "WEB-3",
      "WEB-10",
    ]);
  });
});

describe("ai_37 the names of agents", () => {
  it("orders numbers in names as numbers", () => {
    const all = [
      testAgent("Fixer 10", "active"),
      testAgent("Fixer 3", "active"),
      testAgent("Fixer", "active"),
    ];
    expect(agentsInOrder(all).map((one) => one.name)).toEqual(["Fixer", "Fixer 3", "Fixer 10"]);
  });
});

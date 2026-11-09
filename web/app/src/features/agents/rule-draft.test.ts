import { agentRuleDaysMost } from "@bugaboo/api-client";
import { describe, expect, it, vi } from "vitest";

// The areas the window builds on read a stored choice when they are loaded; there is no browser here.
vi.stubGlobal("localStorage", {
  getItem: vi.fn<(key: string) => string | null>(),
  setItem: vi.fn<(key: string, value: string) => void>(),
});

const { emptyDraft } = await import("../agent-work/index.ts");
const { daysAreRight, emptyRuleDraft, ruleBody, ruleIsComplete, ruleWaitsFor } =
  await import("./rule-draft.ts");

const complete = emptyRuleDraft({ search: "s1", agent: "a1", project: "p1" });
const handing = { ...complete, way: "hand-over" } as const;

describe("m7_wp14 the way of a standing rule", () => {
  it("proposes at the start and sends nothing of a hand-over", () => {
    expect(complete.way).toBe("propose");
    const body = ruleBody({
      ...complete,
      handOver: { ...emptyDraft, changeStatus: true, changeFields: true, fields: ["title"] },
    });
    expect(body).toMatchObject({
      way: "propose",
      powers: [],
      fields: [],
      accept_by_itself: false,
    });
  });

  it("m7_wp14_a_rule_that_hands_over_sends_the_powers_the_fields_and_accepting_by_itself", () => {
    const body = ruleBody({
      ...handing,
      handOver: {
        ...emptyDraft,
        changeFields: true,
        changeStatus: true,
        fields: ["title", "priority"],
        acceptByItself: true,
      },
    });
    expect(body).toMatchObject({
      way: "hand-over",
      powers: ["change-fields", "change-status"],
      fields: ["title", "priority"],
      accept_by_itself: true,
    });
    // Fields are sent only when changing fields is ticked.
    expect(ruleBody({ ...handing, handOver: { ...emptyDraft, fields: ["title"] } }).fields).toEqual(
      [],
    );
  });

  it("the old option to change only some fields is gone", () => {
    expect(Object.keys(complete)).not.toContain("power");
    expect(ruleBody(complete)).not.toHaveProperty("power");
  });
});

describe("ai_32 the rest of the window of a standing rule", () => {
  it("ai_32_a_standing_rule_ends_at_most_90_days_ahead", () => {
    expect(agentRuleDaysMost).toBe(90);
    expect(daysAreRight("")).toBe(true);
    expect(daysAreRight("30")).toBe(true);
    expect(daysAreRight("90")).toBe(true);
    expect(daysAreRight("91")).toBe(false);
    expect(daysAreRight("0")).toBe(false);
    expect(daysAreRight("ten")).toBe(false);
    expect(ruleIsComplete({ ...complete, days: "91" })).toBe(false);
    expect(ruleBody({ ...complete, days: "45" })).toMatchObject({ days: 45 });
    expect(ruleBody(complete)).not.toHaveProperty("days");
  });

  it("m7_wp14_a_rule_that_never_runs_out_sends_no_days", () => {
    const never = { ...complete, neverEnds: true, days: "45" };
    expect(ruleBody(never)).toMatchObject({ never_ends: true });
    expect(ruleBody(never)).not.toHaveProperty("days");
    // The days do not matter then, whatever was typed.
    expect(ruleIsComplete({ ...never, days: "ten" })).toBe(true);
    expect(ruleBody(complete)).not.toHaveProperty("never_ends");
  });

  it("needs a search, an agent and a project, and a field when a hand-over changes fields", () => {
    expect(ruleIsComplete(complete)).toBe(true);
    expect(ruleIsComplete({ ...complete, search: "" })).toBe(false);
    expect(ruleIsComplete({ ...complete, agent: "" })).toBe(false);
    expect(ruleIsComplete({ ...complete, project: "" })).toBe(false);
    expect(ruleIsComplete({ ...handing, handOver: { ...emptyDraft, changeFields: true } })).toBe(
      false,
    );
    expect(
      ruleIsComplete({
        ...handing,
        handOver: { ...emptyDraft, changeFields: true, fields: ["title"] },
      }),
    ).toBe(true);
    expect(ruleIsComplete({ ...complete, createIssues: true })).toBe(false);
    expect(ruleBody({ ...complete, createIssues: true, createsIn: "p2" })).toMatchObject({
      creates_in: "p2",
    });
  });
});

describe("ai_32 the line that says why the window of a standing rule waits", () => {
  it("says why *Create the rule* waits, the first thing missing first", () => {
    expect(ruleWaitsFor(complete)).toBeUndefined();
    expect(ruleWaitsFor(emptyRuleDraft())).toBe("agents.rule-need-search-agent");
    expect(ruleWaitsFor({ ...complete, search: "" })).toBe("agents.rule-need-search");
    // Opened from a saved search, the search is there and only the agent is missing.
    expect(ruleWaitsFor({ ...complete, agent: "" })).toBe("agents.rule-need-agent");
    expect(ruleWaitsFor({ ...complete, project: "" })).toBe("agents.rule-need-project");
    expect(ruleWaitsFor({ ...handing, handOver: { ...emptyDraft, changeFields: true } })).toBe(
      "agents.rule-need-fields",
    );
    expect(ruleWaitsFor({ ...complete, createIssues: true })).toBe("agents.rule-need-creates-in");
    expect(ruleWaitsFor({ ...complete, days: "91" })).toBe("agents.rule-need-days");
    // The window may be confirmed exactly when nothing is missing.
    for (const draft of [complete, emptyRuleDraft(), { ...complete, days: "0" }]) {
      expect(ruleIsComplete(draft)).toBe(ruleWaitsFor(draft) === undefined);
    }
  });
});

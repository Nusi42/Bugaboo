import { agentRuleDaysMost, ruleFieldWords } from "@bugaboo/api-client";
import { describe, expect, it } from "vitest";

import {
  daysAreRight,
  emptyRuleDraft,
  offeredFields,
  ruleBody,
  ruleIsComplete,
  withField,
} from "./rule-draft.ts";

const complete = emptyRuleDraft({ search: "s1", agent: "a1", project: "p1" });

describe("ai_32 the fields of a standing rule", () => {
  it("ai_32_a_standing_rule_offers_only_the_fields_of_the_short_list_and_never_people_or_the_status", () => {
    expect(offeredFields).toEqual(ruleFieldWords);
    for (const forbidden of ["assigned-to", "followers", "reported-by", "status"]) {
      expect(offeredFields).not.toContain(forbidden);
    }
    // Whatever the window sends is in the list.
    const body = ruleBody({ ...complete, power: "change", fields: ["labels", "tags"] });
    for (const field of body.fields) {
      expect(offeredFields).toContain(field);
    }
  });
});

describe("ai_32 the rest of the window of a standing rule", () => {
  it("only proposes at the start and sends no field", () => {
    expect(complete.power).toBe("propose");
    expect(ruleBody({ ...complete, fields: ["labels"] }).fields).toEqual([]);
  });

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

  it("needs a search, an agent and a project, and a field when it changes fields", () => {
    expect(ruleIsComplete(complete)).toBe(true);
    expect(ruleIsComplete({ ...complete, search: "" })).toBe(false);
    expect(ruleIsComplete({ ...complete, agent: "" })).toBe(false);
    expect(ruleIsComplete({ ...complete, project: "" })).toBe(false);
    expect(ruleIsComplete({ ...complete, power: "change" })).toBe(false);
    expect(ruleIsComplete({ ...complete, power: "change", fields: ["tags"] })).toBe(true);
    expect(ruleIsComplete({ ...complete, createIssues: true })).toBe(false);
    expect(ruleBody({ ...complete, createIssues: true, createsIn: "p2" })).toMatchObject({
      creates_in: "p2",
    });
  });

  it("keeps the order of the list when a field is ticked", () => {
    expect(withField(["tags"], "labels", true)).toEqual(["labels", "tags"]);
    expect(withField(["labels", "tags"], "labels", false)).toEqual(["tags"]);
  });
});

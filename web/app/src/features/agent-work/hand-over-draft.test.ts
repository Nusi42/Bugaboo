import { describe, expect, it } from "vitest";

import { deviceRegion, setRegion } from "../../shared/region.ts";
import {
  emptyDraft,
  handOverBody,
  nameableFields,
  numbersAreRight,
  wholeNumber,
} from "./hand-over-draft.ts";

describe("ai_31 the draft of a hand-over", () => {
  it("sends nothing the person did not tick, and no field without the tick for fields", () => {
    const body = handOverBody(
      { ...emptyDraft, agent: "A1", note: "Look at it", fields: ["labels"] },
      ["WEB-1"],
    );
    expect(body.powers).toEqual([]);
    expect(body.fields).toEqual([]);
    expect(body.issues).toEqual(["WEB-1"]);
    expect(body).not.toHaveProperty("budget_attempts");
    expect(body.plan_needs_approval).toBe(false);
  });

  it("sends the ticks, the named fields and the budget that was typed", () => {
    const body = handOverBody(
      {
        ...emptyDraft,
        agent: "A1",
        changeFields: true,
        changeStatus: true,
        fields: ["labels", "priority"],
        attempts: "3",
        minutes: " 120 ",
        days: "7",
        planNeedsApproval: true,
      },
      ["WEB-1", "WEB-2"],
    );
    expect(body.powers).toEqual(["change-fields", "change-status"]);
    expect(body.fields).toEqual(["labels", "priority"]);
    expect(body.budget_attempts).toBe(3);
    expect(body.budget_minutes).toBe(120);
    expect(body.days).toBe(7);
    expect(body.plan_needs_approval).toBe(true);
  });

  it("reads whole numbers only", () => {
    expect(wholeNumber("12")).toBe(12);
    expect(wholeNumber("")).toBeUndefined();
    expect(wholeNumber("1.5")).toBeUndefined();
    expect(numbersAreRight({ ...emptyDraft, cost: "x" })).toBe(false);
    expect(numbersAreRight(emptyDraft)).toBe(true);
  });
});

describe("ai_31 the cost budget of a hand-over", () => {
  it("takes the cost in euros as the person writes numbers and sends cents", () => {
    setRegion({ ...deviceRegion, numberFormat: "comma" });
    expect(handOverBody({ ...emptyDraft, agent: "A1", cost: "12,50" }, ["WEB-1"]).budget_cost).toBe(
      1250,
    );
    expect(numbersAreRight({ ...emptyDraft, cost: "12,505" })).toBe(false);
    setRegion({ ...deviceRegion, numberFormat: "point" });
    expect(handOverBody({ ...emptyDraft, agent: "A1", cost: "3" }, ["WEB-1"]).budget_cost).toBe(
      300,
    );
    setRegion(deviceRegion);
  });
});

type OwnFieldKind = Parameters<typeof nameableFields>[0][number]["kind"];

const own = (
  id: string,
  kind: OwnFieldKind,
  appliesTo: "issues" | "projects",
  retired = false,
) => ({
  id,
  name: `Field ${id}`,
  kind,
  applies_to: appliesTo,
  retired,
});

describe("ai_31 what an agent can be given to change", () => {
  it("offers every member the server lists and the own fields of issues that hold no people", () => {
    const codes = nameableFields([
      own("F1", "text", "issues"),
      own("F2", "person", "issues"),
      own("F3", "people", "issues"),
      own("F4", "text", "projects"),
      own("F5", "text", "issues", true),
    ]).map((field) => field.code);
    expect(codes).toEqual(
      expect.arrayContaining(["title", "relations", "comments", "attachments"]),
    );
    expect(codes.at(-1)).toBe("F1");
    expect(codes).not.toContain("F2");
    expect(codes).not.toContain("F3");
    expect(codes).not.toContain("F4");
    expect(codes).not.toContain("F5");
  });
});

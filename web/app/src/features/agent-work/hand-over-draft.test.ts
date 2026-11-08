import { describe, expect, it } from "vitest";

import { emptyDraft, handOverBody, numbersAreRight, wholeNumber } from "./hand-over-draft.ts";

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

import { describe, expect, it } from "vitest";

import { outcomeOf, reviewOf } from "./outcome.ts";

const by = { id: "P1", name: "Ada" };

describe("ai_35 how work ended", () => {
  it("says accepted, accepted by itself and thrown away once the work is over", () => {
    expect(outcomeOf({ open: false, reviews: [{ answer: "accept", by }] })).toBe("accepted");
    expect(outcomeOf({ open: false, reviews: [{ answer: "accept", by: null }] })).toBe(
      "accepted-by-itself",
    );
    expect(outcomeOf({ open: false, reviews: [{ answer: "throw-away", by }] })).toBe("thrown-away");
  });

  it("says nothing for a send-back, for work that is open and for work with no review", () => {
    expect(outcomeOf({ open: false, reviews: [{ answer: "send-back", by }] })).toBeUndefined();
    expect(outcomeOf({ open: true, reviews: [{ answer: "accept", by }] })).toBeUndefined();
    expect(outcomeOf({ open: false, reviews: [] })).toBeUndefined();
  });

  it("gives each review to the attempt before it, in order", () => {
    const work = {
      attempts: [{ id: "A1" }, { id: "A2" }],
      reviews: [{ answer: "send-back" }],
    };
    expect(reviewOf(work, { id: "A1" })?.answer).toBe("send-back");
    expect(reviewOf(work, { id: "A2" })).toBeUndefined();
  });
});

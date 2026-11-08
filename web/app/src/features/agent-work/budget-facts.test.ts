import { describe, expect, it } from "vitest";

import { deviceRegion, setRegion } from "../../shared/region.ts";
import { budgetFacts } from "./budget-facts.ts";

describe("ai_31 the budget of a piece of work in words", () => {
  it("says no attempt yet, and the time and the cost against their limits", () => {
    setRegion({ ...deviceRegion, numberFormat: "point" });
    const budget = { attempts: 3, minutes: 120, cost: 500 };
    expect(budgetFacts({ budget, attempts: [] }).map((fact) => fact.node)).toEqual([
      "No attempt yet",
      "0m of 2h",
      "€0.00 of €5.00",
    ]);
  });

  it("says the attempt the work is at, and names no limit nobody set", () => {
    const attempts = [
      { reported_minutes: 25, reported_cost: 40 },
      { reported_minutes: 15, reported_cost: 0 },
    ];
    expect(
      budgetFacts({ budget: { attempts: 3, minutes: null, cost: null }, attempts }).map(
        (fact) => fact.node,
      ),
    ).toEqual(["Attempt 2 of 3"]);
    expect(
      budgetFacts({ budget: { attempts: 3, minutes: 120 }, attempts }).map((fact) => fact.node),
    ).toEqual(["Attempt 2 of 3", "40m of 2h"]);
  });
});

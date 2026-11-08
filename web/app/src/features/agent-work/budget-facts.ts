import { t } from "@bugaboo/messages";
import type { Fact } from "@bugaboo/ui";

import { minutesText } from "../../shared/own-value-text.ts";
import { amountText } from "./agent-work.ts";

/** What of a piece of work the budget line reads. */
export interface BudgetOf {
  readonly budget: {
    readonly attempts: number;
    readonly minutes?: number | null;
    readonly cost?: number | null;
  };
  readonly attempts: readonly {
    readonly reported_minutes: number;
    readonly reported_cost: number;
  }[];
}

/**
 * The budget as far as the hand-over limits it and the agent reports it (UX §4.17): the attempt the
 * work is at of the most it may make, the time the attempts reported of the most minutes, and the
 * cost of the most cost. A limit nobody set is not named.
 */
export function budgetFacts({ budget, attempts }: BudgetOf): readonly Fact[] {
  const used = {
    minutes: attempts.reduce((sum, attempt) => sum + attempt.reported_minutes, 0),
    cost: attempts.reduce((sum, attempt) => sum + attempt.reported_cost, 0),
  };
  return [
    {
      key: "attempts",
      node:
        attempts.length === 0
          ? t("agent-work.fact-no-attempt")
          : t("agent-work.fact-attempts", { number: attempts.length, most: budget.attempts }),
    },
    ...(budget.minutes === null || budget.minutes === undefined
      ? []
      : [
          {
            key: "minutes",
            node: t("agent-work.fact-of", {
              used: minutesText(used.minutes),
              most: minutesText(budget.minutes),
            }),
          },
        ]),
    ...(budget.cost === null || budget.cost === undefined
      ? []
      : [
          {
            key: "cost",
            node: t("agent-work.fact-of", {
              used: amountText(used.cost),
              most: amountText(budget.cost),
            }),
          },
        ]),
  ];
}

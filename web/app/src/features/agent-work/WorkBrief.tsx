import { t } from "@bugaboo/messages";
import { Facts, partOf, part, type Fact } from "@bugaboo/ui";
import { useId } from "react";

import { minutesText } from "../../shared/own-value-text.ts";
import { centsText, type Work } from "./agent-work.ts";

/** What the attempts reported so far: the attempts, the minutes and the cost, against the budget. */
function factsOf(work: Work): readonly Fact[] {
  const minutes = work.attempts.reduce((sum, attempt) => sum + attempt.reported_minutes, 0);
  const cost = work.attempts.reduce((sum, attempt) => sum + attempt.reported_cost, 0);
  const facts: Fact[] = [
    {
      key: "attempts",
      node: t("agent-work.fact-attempts", {
        number: work.attempts.length,
        most: work.budget_attempts,
      }),
    },
  ];
  if (minutes > 0 || work.budget_minutes !== null) {
    facts.push({
      key: "time",
      node:
        work.budget_minutes === null || work.budget_minutes === undefined
          ? t("agent-work.fact-time", { time: minutesText(minutes) })
          : t("agent-work.fact-time-of", {
              time: minutesText(minutes),
              most: minutesText(work.budget_minutes),
            }),
    });
  }
  if (cost > 0 || work.budget_cost !== null) {
    facts.push({
      key: "cost",
      node:
        work.budget_cost === null || work.budget_cost === undefined
          ? t("agent-work.fact-cost", { amount: centsText(cost) })
          : t("agent-work.fact-cost-of", {
              amount: centsText(cost),
              most: centsText(work.budget_cost),
            }),
    });
  }
  return facts;
}

/**
 * The brief the agent works from, the answers that joined it, and the budget as far as the agent
 * reports it. What the agent reports is not a time entry and counts in no total (M7-Q7).
 */
export function WorkBrief({ work }: { readonly work: Work }) {
  const heading = useId();
  return (
    <section {...partOf(part.agentWorkSection)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.agentWorkLabel)}>
        {t("agent-work.brief")}
      </h3>
      {work.brief.length === 0 ? (
        <p {...partOf(part.muted)}>{t("agent-work.brief-empty")}</p>
      ) : (
        work.brief.map((text) => (
          <p key={text} {...partOf(part.agentWorkWords)}>
            {text}
          </p>
        ))
      )}
      <p {...partOf(part.muted)}>
        <Facts items={factsOf(work)} />
      </p>
      {work.attempts.some(
        (attempt) => attempt.reported_minutes > 0 || attempt.reported_cost > 0,
      ) ? (
        <p {...partOf(part.muted)}>{t("agent-work.reported-note")}</p>
      ) : null}
    </section>
  );
}

import { t } from "@bugaboo/messages";
import { Button, buttonLook, Facts, partOf, part, type Fact } from "@bugaboo/ui";
import { useId, useState } from "react";

import { listText } from "../../shared/list-text.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { amountText, type BriefLack, type Work } from "./agent-work.ts";

/** What the attempts reported so far: the attempts, the minutes and the cost, against the budget. */
function factsOf(work: Work): readonly Fact[] {
  const minutes = work.attempts.reduce((sum, attempt) => sum + attempt.reported_minutes, 0);
  const cost = work.attempts.reduce((sum, attempt) => sum + attempt.reported_cost, 0);
  const { budget } = work;
  const facts: Fact[] = [
    {
      key: "attempts",
      node: t("agent-work.fact-attempts", { number: work.attempts.length, most: budget.attempts }),
    },
  ];
  if (minutes > 0 || (budget.minutes !== null && budget.minutes !== undefined)) {
    facts.push({
      key: "time",
      node:
        budget.minutes === null || budget.minutes === undefined
          ? t("agent-work.fact-time", { time: minutesText(minutes) })
          : t("agent-work.fact-time-of", {
              time: minutesText(minutes),
              most: minutesText(budget.minutes),
            }),
    });
  }
  if (cost > 0 || (budget.cost !== null && budget.cost !== undefined)) {
    facts.push({
      key: "cost",
      node:
        budget.cost === null || budget.cost === undefined
          ? t("agent-work.fact-cost", { amount: amountText(cost) })
          : t("agent-work.fact-cost-of", {
              amount: amountText(cost),
              most: amountText(budget.cost),
            }),
    });
  }
  return facts;
}

/** The parts of the brief the agent has: what the issue holds of the three, and the note with the answers. */
function partsOf(work: Work, lacking: readonly BriefLack[]): string[] {
  const words = {
    description: t("agent-work.part-description"),
    "steps-to-reproduce": t("agent-work.part-steps-to-reproduce"),
    "done-when": t("agent-work.part-done-when"),
  } as const;
  const held = (["description", "steps-to-reproduce", "done-when"] as const)
    .filter((each) => !lacking.includes(each))
    .map((each) => words[each]);
  return work.brief.length === 0 ? held : [...held, t("agent-work.part-note")];
}

/**
 * The brief the agent works from, as its parts (*Description, Done when, Note*) with *Show* for the
 * texts of the note and the answers that joined it, and the budget as far as the agent reports it.
 * What the agent reports is not a time entry and counts in no total (M7-Q7).
 */
export function WorkBrief({
  work,
  lacking,
}: {
  readonly work: Work;
  /** The parts of the brief the issue lacks now. */
  readonly lacking: readonly BriefLack[];
}) {
  const heading = useId();
  const [shown, setShown] = useState(false);
  const parts = partsOf(work, lacking);
  return (
    <section {...partOf(part.agentWorkSection)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.agentWorkLabel)}>
        {t("agent-work.brief")}
      </h3>
      <p {...partOf(part.agentWorkParts)}>
        <span>{parts.length === 0 ? t("agent-work.brief-empty") : listText(parts)}</span>
        {work.brief.length === 0 ? null : (
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              setShown(!shown);
            }}
          >
            {shown ? t("agent-work.brief-hide") : t("agent-work.brief-show")}
          </Button>
        )}
      </p>
      {shown
        ? work.brief.map((text) => (
            <p key={text} {...partOf(part.agentWorkWords)}>
              {text}
            </p>
          ))
        : null}
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

import { t } from "@bugaboo/messages";
import { Button, buttonLook, Facts, partOf, part, type Fact } from "@bugaboo/ui";
import { useId, useState } from "react";

import { listText } from "../../shared/list-text.ts";
import type { BriefLack, Work } from "./agent-work.ts";

/** The attempts against the attempts the hand-over allows; the time and the cost stand on each attempt. */
function factsOf(work: Work): readonly Fact[] {
  return [
    {
      key: "attempts",
      node: t("agent-work.fact-attempts", {
        number: work.attempts.length,
        most: work.budget.attempts,
      }),
    },
  ];
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
    </section>
  );
}

import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { runAddress } from "../../shared/paths.ts";
import type { Attempt, Work } from "./agent-work.ts";
import { AttemptHead } from "./AttemptHead.tsx";
import { PlanApproval, PlanSteps } from "./PlanLines.tsx";
import { ReportedLine } from "./ReportedLine.tsx";
import { Results } from "./Results.tsx";
import { ReviewOfAttempt } from "./Reviews.tsx";

/**
 * One attempt (UX §4.17): its number, how it ended and when it began, the plan with *Approve plan*
 * under it, the results (links, then checks), the time and cost *as reported by the agent*, the
 * review that answered it with its note, and the link to its run, which lists what it changed.
 */
export function AttemptLines({
  work,
  attempt,
}: {
  readonly work: Work;
  readonly attempt: Attempt;
}) {
  const heading = useId();
  return (
    <section {...partOf(part.agentWorkAttempt)} aria-labelledby={heading}>
      <AttemptHead id={heading} work={work} attempt={attempt} />
      <PlanSteps attempt={attempt} />
      <Results attempt={attempt} />
      <ReportedLine attempt={attempt} />
      <PlanApproval work={work} attempt={attempt} />
      <ReviewOfAttempt work={work} attempt={attempt} />
      {attempt.run === null || attempt.run === undefined ? null : (
        <p {...partOf(part.text)}>
          <Link href={runAddress(attempt.run)}>{t("agent-work.run-link")}</Link>
        </p>
      )}
    </section>
  );
}

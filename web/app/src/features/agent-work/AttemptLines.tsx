import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { runAddress } from "../../shared/paths.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { centsText, type Attempt, type Work } from "./agent-work.ts";
import { Results } from "./Results.tsx";

/** Whether the plan of the attempt is still waiting for a person (the hand-over asks for approval). */
function planWaits(work: Work, attempt: Attempt): boolean {
  return work.plan_needs_approval && attempt.plan !== null && !attempt.plan_approved;
}

/** The plan the agent reported and whether a person approved it. */
function PlanLines({ work, attempt }: { readonly work: Work; readonly attempt: Attempt }) {
  if (attempt.plan === null || attempt.plan === undefined) {
    return <p {...partOf(part.muted)}>{t("agent-work.no-plan")}</p>;
  }
  return (
    <div {...partOf(part.agentWorkSection)}>
      <p {...partOf(part.agentWorkWords)}>{attempt.plan}</p>
      <p {...partOf(part.muted)}>
        {planWaits(work, attempt) ? t("agent-work.plan-waits") : null}
        {attempt.plan_approved ? t("agent-work.plan-approved") : null}
      </p>
    </div>
  );
}

/**
 * One attempt (UX §4.17): its number and when it began, the plan, the results as links and checks,
 * the time and cost *as reported by the agent*, and the link to its run, which lists what it changed.
 */
export function AttemptLines({
  work,
  attempt,
}: {
  readonly work: Work;
  readonly attempt: Attempt;
}) {
  const heading = useId();
  const when =
    attempt.ended_at === null || attempt.ended_at === undefined
      ? t("agent-work.attempt-started", { date: formatDateTime(attempt.started_at) })
      : t("agent-work.attempt-ended", { date: formatDateTime(attempt.ended_at) });
  const reported = attempt.reported_minutes > 0 || attempt.reported_cost > 0;
  return (
    <section {...partOf(part.agentWorkAttempt)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.agentWorkLabel)}>
        {t("agent-work.attempt", { number: attempt.number })}
      </h3>
      <p {...partOf(part.muted)}>{when}</p>
      <PlanLines work={work} attempt={attempt} />
      <Results attempt={attempt} />
      {reported ? (
        <p {...partOf(part.muted)}>
          {t("agent-work.attempt-reported", {
            time: minutesText(attempt.reported_minutes),
            cost: centsText(attempt.reported_cost),
          })}
        </p>
      ) : null}
      {attempt.run === null || attempt.run === undefined ? null : (
        <p {...partOf(part.text)}>
          <Link href={runAddress(attempt.run)}>{t("agent-work.run-link")}</Link>
        </p>
      )}
    </section>
  );
}

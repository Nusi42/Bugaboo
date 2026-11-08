import { t } from "@bugaboo/messages";
import { Facts, Length, Link, partOf, part } from "@bugaboo/ui";
import { useId } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { minutesText } from "../../shared/own-value-text.ts";
import { runAddress } from "../../shared/paths.ts";
import { amountText, lastAttempt, planWaitsForYou, type Attempt, type Work } from "./agent-work.ts";
import { ApprovePlan } from "./ApprovePlan.tsx";
import { planSteps } from "./plan-steps.ts";
import { Results } from "./Results.tsx";

/** The plan the agent reported as numbered steps, and whether a person approved it. */
function PlanLines({ work, attempt }: { readonly work: Work; readonly attempt: Attempt }) {
  if (attempt.plan === null || attempt.plan === undefined) {
    return <p {...partOf(part.muted)}>{t("agent-work.no-plan")}</p>;
  }
  const waits = planWaitsForYou(work) && lastAttempt(work)?.id === attempt.id;
  return (
    <div {...partOf(part.agentWorkSection)}>
      <ol {...partOf(part.agentWorkSteps)}>
        {planSteps(attempt.plan).map((step) => (
          <li key={step}>{step}</li>
        ))}
      </ol>
      {attempt.plan_approved ? (
        <p {...partOf(part.muted)}>{t("agent-work.plan-approved")}</p>
      ) : null}
      {waits ? <p {...partOf(part.muted)}>{t("agent-work.plan-waits")}</p> : null}
      {waits ? <ApprovePlan work={work} /> : null}
    </div>
  );
}

/** The time and the cost the agent reported, in one line: the time in the code font, then the cost. */
function ReportedLine({ attempt }: { readonly attempt: Attempt }) {
  if (attempt.reported_minutes <= 0 && attempt.reported_cost <= 0) {
    return null;
  }
  return (
    <p {...partOf(part.muted)}>
      <Facts
        items={[
          { key: "time", node: <Length text={minutesText(attempt.reported_minutes)} /> },
          {
            key: "cost",
            node: t("agent-work.attempt-reported", { amount: amountText(attempt.reported_cost) }),
          },
        ]}
      />
    </p>
  );
}

/**
 * One attempt (UX §4.17): its number and when it began, the plan with *Approve plan* under it, the
 * results (links, then checks), the time and cost *as reported by the agent*, and the link to its
 * run, which lists what it changed.
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
  return (
    <section {...partOf(part.agentWorkAttempt)} aria-labelledby={heading}>
      <h3 id={heading} {...partOf(part.agentWorkLabel)}>
        {t("agent-work.attempt", { number: attempt.number })}
      </h3>
      <p {...partOf(part.muted)}>{when}</p>
      <PlanLines work={work} attempt={attempt} />
      <Results attempt={attempt} />
      <ReportedLine attempt={attempt} />
      {attempt.run === null || attempt.run === undefined ? null : (
        <p {...partOf(part.text)}>
          <Link href={runAddress(attempt.run)}>{t("agent-work.run-link")}</Link>
        </p>
      )}
    </section>
  );
}

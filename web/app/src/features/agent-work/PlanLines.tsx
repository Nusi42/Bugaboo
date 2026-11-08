import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { lastAttempt, planWaitsForYou, type Attempt, type Work } from "./agent-work.ts";
import { ApprovePlan } from "./ApprovePlan.tsx";
import { planSteps } from "./plan-steps.ts";

/** The plan the agent reported as numbered steps, and that it is approved. */
export function PlanSteps({ attempt }: { readonly attempt: Attempt }) {
  if (attempt.plan === null || attempt.plan === undefined) {
    return <p {...partOf(part.muted)}>{t("agent-work.no-plan")}</p>;
  }
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
    </div>
  );
}

/** What waits under the plan for the person: the words that it waits, and *Approve plan* with its note. */
export function PlanApproval({
  work,
  attempt,
}: {
  readonly work: Work;
  readonly attempt: Attempt;
}) {
  const waits = planWaitsForYou(work) && lastAttempt(work)?.id === attempt.id;
  if (!waits) {
    return null;
  }
  return (
    <div {...partOf(part.agentWorkSection)}>
      <p {...partOf(part.muted)}>{t("agent-work.plan-waits")}</p>
      <ApprovePlan work={work} />
    </div>
  );
}

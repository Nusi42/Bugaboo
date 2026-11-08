import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { lastAttempt, planWaitsForYou, type Attempt, type Work } from "./agent-work.ts";
import { ApprovePlan } from "./ApprovePlan.tsx";
import { planSteps } from "./plan-steps.ts";

/** The plan the agent reported as numbered steps, and whether a person approved it. */
export function PlanLines({ work, attempt }: { readonly work: Work; readonly attempt: Attempt }) {
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

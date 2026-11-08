import type { Schema } from "@bugaboo/api-client";
import { partOf, part } from "@bugaboo/ui";

import { AcceptedByItself } from "./AcceptedByItself.tsx";
import { isReadyForReview, planWaitsForYou, type Work } from "./agent-work.ts";
import { ApprovePlan } from "./ApprovePlan.tsx";
import { ReviewButton } from "./ReviewButton.tsx";

/** What waits for the person on a piece of work: the plan to approve, the review, the result accepted by itself. */
export function WorkActions({
  work,
  issue,
  statusChanges,
}: {
  readonly work: Work;
  readonly issue: string;
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
}) {
  return (
    <>
      {planWaitsForYou(work) ? <ApprovePlan work={work} /> : null}
      {isReadyForReview(work) ? (
        <div {...partOf(part.agentWorkActions)}>
          <ReviewButton work={work} issue={issue} statusChanges={statusChanges} />
        </div>
      ) : null}
      <AcceptedByItself work={work} />
    </>
  );
}

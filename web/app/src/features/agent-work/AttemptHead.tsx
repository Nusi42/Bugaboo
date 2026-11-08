import { t } from "@bugaboo/messages";
import { factSeparator, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import type { Attempt, Work } from "./agent-work.ts";
import { attemptOutcomeText, reviewOf } from "./outcome.ts";

/**
 * The heading of an attempt and when it began or ended. The heading names how the attempt ended once
 * a review answered it (*Attempt 1 · sent back*).
 */
export function AttemptHead({
  id,
  work,
  attempt,
}: {
  /** The ID the section of the attempt is labelled by. */
  readonly id: string;
  readonly work: Work;
  readonly attempt: Attempt;
}) {
  const review = reviewOf(work, attempt);
  const name = t("agent-work.attempt", { number: attempt.number });
  const title =
    review === undefined ? name : [name, factSeparator, attemptOutcomeText(review)].join(" ");
  const when =
    attempt.ended_at === null || attempt.ended_at === undefined
      ? t("agent-work.attempt-started", { date: formatDateTime(attempt.started_at) })
      : t("agent-work.attempt-ended", { date: formatDateTime(attempt.ended_at) });
  return (
    <>
      <h3 id={id} {...partOf(part.agentWorkLabel)}>
        {title}
      </h3>
      <p {...partOf(part.muted)}>{when}</p>
    </>
  );
}

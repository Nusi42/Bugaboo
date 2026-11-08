import { t } from "@bugaboo/messages";
import { factSeparator, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import type { Work } from "./agent-work.ts";
import { reviewOf } from "./outcome.ts";

type Review = Work["reviews"][number];

/** What a review was, in words: who answered what. Nobody answered a result accepted by itself. */
function reviewText(review: Review): string {
  if (review.by === null || review.by === undefined) {
    return t("agent-work.review-accept-itself");
  }
  const person = personName(review.by);
  if (review.answer === "accept") {
    return t("agent-work.review-accept", { person });
  }
  return review.answer === "send-back"
    ? t("agent-work.review-send-back", { person })
    : t("agent-work.review-throw-away", { person });
}

/** The review of an attempt: who answered what and when, and the note that came with it. */
export function ReviewLine({ review }: { readonly review: Review }) {
  return (
    <div {...partOf(part.agentWorkSection)}>
      <p {...partOf(part.text)}>
        {[reviewText(review), factSeparator, formatDateTime(review.at)].join(" ")}
      </p>
      {review.note === null || review.note === undefined ? null : (
        <p {...partOf(part.agentWorkWords)}>{t("agent-work.review-note", { note: review.note })}</p>
      )}
    </div>
  );
}

/** The review that answered an attempt, with its note; nothing for an attempt no review answered. */
export function ReviewOfAttempt({
  work,
  attempt,
}: {
  readonly work: Work;
  readonly attempt: Work["attempts"][number];
}) {
  const review = reviewOf(work, attempt);
  return review === undefined ? null : <ReviewLine review={review} />;
}

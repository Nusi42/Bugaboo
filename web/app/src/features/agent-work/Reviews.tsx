import { t } from "@bugaboo/messages";
import { factSeparator, partOf, part } from "@bugaboo/ui";

import { formatDateTime } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import type { Work } from "./agent-work.ts";

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

/** The line of a review: what it was and when. */
function lineOf(review: Review): string {
  return [reviewText(review), factSeparator, formatDateTime(review.at)].join(" ");
}

/** What tells a review from the others of its work. */
function keyOf(review: Review): string {
  return [review.at, review.answer].join("/");
}

/** The reviews of the work, oldest first, each with its note. */
export function Reviews({ work }: { readonly work: Work }) {
  if (work.reviews.length === 0) {
    return null;
  }
  return (
    <ul {...partOf(part.agentWorkResults)} aria-label={t("agent-work.reviews")}>
      {work.reviews.map((review) => (
        <li key={keyOf(review)}>
          <span>{lineOf(review)}</span>
          {review.note === null || review.note === undefined ? null : (
            <p {...partOf(part.agentWorkWords)}>
              {t("agent-work.review-note", { note: review.note })}
            </p>
          )}
        </li>
      ))}
    </ul>
  );
}

import { t } from "@bugaboo/messages";

/** How a piece of work that is over ended by a person's review, or by itself. */
export type Outcome = "accepted" | "accepted-by-itself" | "thrown-away";

/** What of a review the words below read: what it decided and who answered (nobody for a result accepted by itself). */
export interface ReviewOf {
  readonly answer: "accept" | "send-back" | "throw-away";
  readonly by?: unknown;
}

/** What a review decided, in the words of an outcome; a send-back is no end of the work. */
export function reviewOutcome(review: ReviewOf): Outcome | "sent-back" {
  if (review.answer === "send-back") {
    return "sent-back";
  }
  if (review.answer === "throw-away") {
    return "thrown-away";
  }
  return review.by === null || review.by === undefined ? "accepted-by-itself" : "accepted";
}

/** How the work ended once it is over by a review; none while it is open or ended without one (gave up, stopped). */
export function outcomeOf(work: {
  readonly open: boolean;
  readonly reviews: readonly ReviewOf[];
}): Outcome | undefined {
  const last = work.reviews.at(-1);
  if (work.open || last === undefined) {
    return undefined;
  }
  const outcome = reviewOutcome(last);
  return outcome === "sent-back" ? undefined : outcome;
}

/** The review that answered an attempt: each review ends the attempt that came before it, in order. */
export function reviewOf<Review>(
  work: {
    readonly attempts: readonly { readonly id: string }[];
    readonly reviews: readonly Review[];
  },
  attempt: { readonly id: string },
): Review | undefined {
  const at = work.attempts.findIndex((each) => each.id === attempt.id);
  return at < 0 ? undefined : work.reviews[at];
}

const outcomeWords: Readonly<Record<Outcome, () => string>> = {
  accepted: () => t("agent-work.outcome.accepted"),
  "accepted-by-itself": () => t("agent-work.outcome.accepted-by-itself"),
  "thrown-away": () => t("agent-work.outcome.thrown-away"),
};

/** How the work ended, in a word for the pill: *Accepted*. */
export function outcomeText(outcome: Outcome): string {
  return outcomeWords[outcome]();
}

const attemptWords: Readonly<Record<Outcome | "sent-back", () => string>> = {
  "sent-back": () => t("agent-work.attempt-sent-back"),
  accepted: () => t("agent-work.attempt-accepted"),
  "accepted-by-itself": () => t("agent-work.attempt-accepted-by-itself"),
  "thrown-away": () => t("agent-work.attempt-thrown-away"),
};

/** What a review decided, in a word that goes behind the name of the attempt: *sent back*. */
export function attemptOutcomeText(review: ReviewOf): string {
  return attemptWords[reviewOutcome(review)]();
}

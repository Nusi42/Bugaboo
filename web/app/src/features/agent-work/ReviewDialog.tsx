import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook, partOf, part } from "@bugaboo/ui";

import { lastAttempt, useReviewWork, type Verdict, type Work } from "./agent-work.ts";
import { Results } from "./Results.tsx";
import { ReviewFields } from "./ReviewFields.tsx";
import { useReviewDraft } from "./use-review-draft.ts";

export interface ReviewDialogProps {
  readonly work: Work;
  /** The readable ID of the issue. */
  readonly issue: string;
  /** The status changes the workflow offers the person now. */
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Called with what refused the review; the window is gone by then, the button that opened it says it. */
  readonly onFailed: (error: unknown) => void;
}

const confirmOf = {
  accept: "agent-work.review-confirm-accept",
  "send-back": "agent-work.review-confirm-send-back",
  "throw-away": "agent-work.review-confirm-throw-away",
} as const satisfies Record<Verdict, string>;

/**
 * The review window at *Ready for review* (UX §4.17): *Accept* with the status to move to (the
 * workflow's proposal first), *Send back* with a note that is needed, and *Throw away* with what
 * would go back. The results are listed, and the window says Bugaboo does not verify the checks.
 */
export function ReviewDialog(props: ReviewDialogProps) {
  const { work, issue, isOpen, onOpenChange } = props;
  const draft = useReviewDraft(props.statusChanges);
  const send = useReviewWork(props.onFailed);
  const attempt = lastAttempt(work);
  return (
    <AskDialog
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      title={t("agent-work.review-title", {
        number: attempt?.number ?? work.attempts.length,
        agent: work.agent_name,
        issue,
      })}
      text={t("agent-work.review-text")}
      confirmLabel={t(confirmOf[draft.answer])}
      cancelLabel={t("common.cancel")}
      confirmLook={draft.answer === "throw-away" ? buttonLook.danger : buttonLook.primary}
      confirmDisabled={draft.needsNote || send.isPending}
      {...(draft.needsNote ? { confirmHint: t("agent-work.review-note-required") } : {})}
      onConfirm={() => {
        send.mutate(
          {
            params: { path: { work: work.id } },
            body: { ...draft.body(), version: work.version },
          },
          {
            onSuccess: () => {
              onOpenChange(false);
            },
          },
        );
      }}
    >
      {attempt === undefined ? null : <Results attempt={attempt} />}
      {attempt === undefined || attempt.results.length > 0 ? null : (
        <p {...partOf(part.muted)}>{t("agent-work.review-no-results")}</p>
      )}
      <ReviewFields work={work} draft={draft} />
    </AskDialog>
  );
}

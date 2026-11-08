import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part, Window } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { windowLabels } from "../../shared/window-labels.ts";
import { lastAttempt, useReviewWork, type Attempt, type Verdict, type Work } from "./agent-work.ts";
import { Results } from "./Results.tsx";
import { ReviewFields } from "./ReviewFields.tsx";
import { useReviewDraft, type ReviewDraft } from "./use-review-draft.ts";

export interface ReviewDialogProps {
  readonly work: Work;
  /** The readable ID of the issue. */
  readonly issue: string;
  /** The status changes the workflow offers the person now. */
  readonly statusChanges: readonly Schema<"StatusChangeRef">[] | null | undefined;
  readonly isOpen: boolean;
  readonly onOpenChange: (open: boolean) => void;
  /** Called with what refused the review, besides the window saying it. */
  readonly onFailed?: (error: unknown) => void;
}

const confirmOf = {
  accept: "agent-work.review-confirm-accept",
  "send-back": "agent-work.review-confirm-send-back",
  "throw-away": "agent-work.review-confirm-throw-away",
} as const satisfies Record<Verdict, string>;

/** What the attempt attached, or that it attached nothing. */
function ReviewResults({ attempt }: { readonly attempt: Attempt | undefined }) {
  if (attempt === undefined) {
    return null;
  }
  return attempt.results.length > 0 ? (
    <Results attempt={attempt} />
  ) : (
    <p {...partOf(part.muted)}>{t("agent-work.review-no-results")}</p>
  );
}

/** The buttons of the window: *Cancel*, and the confirmation that is named for what was chosen. */
function ReviewFooter({
  draft,
  isPending,
  onSend,
  onClose,
}: {
  readonly draft: ReviewDraft;
  readonly isPending: boolean;
  readonly onSend: () => void;
  readonly onClose: () => void;
}) {
  return (
    <div {...partOf(part.windowFooterActions)}>
      <Button look={buttonLook.quiet} onPress={onClose}>
        {t("common.cancel")}
      </Button>
      <Button
        look={draft.answer === "throw-away" ? buttonLook.danger : buttonLook.primary}
        isPending={isPending}
        isDisabled={draft.needsNote}
        onPress={onSend}
      >
        {t(confirmOf[draft.answer])}
      </Button>
    </div>
  );
}

/**
 * The review window at *Ready for review* (UX §4.17, §6.3), in the one window part of the app:
 * *Accept* with the status to move to (the workflow's proposal first), *Send back* with a note that
 * is needed, and *Throw away* with what would go back. The results are listed, and the window says
 * Bugaboo does not verify the checks. A refusal is said in the window, which then stays open.
 */
export function ReviewDialog(props: ReviewDialogProps) {
  const { work, issue, isOpen, onOpenChange } = props;
  const draft = useReviewDraft(props.statusChanges);
  const send = useReviewWork(props.onFailed);
  const attempt = lastAttempt(work);
  return (
    <Window
      isOpen={isOpen}
      onOpenChange={onOpenChange}
      labels={windowLabels()}
      title={t("agent-work.review-title", {
        number: attempt?.number ?? work.attempts.length,
        agent: work.agent_name,
        issue,
      })}
      footer={
        <ReviewFooter
          draft={draft}
          isPending={send.isPending}
          onSend={() => {
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
          onClose={() => {
            onOpenChange(false);
          }}
        />
      }
    >
      <div {...partOf(part.stack)}>
        <p {...partOf(part.text)}>{t("agent-work.review-text")}</p>
        <ReviewResults attempt={attempt} />
        <ReviewFields work={work} draft={draft} />
        {draft.needsNote ? (
          <p {...partOf(part.muted)}>{t("agent-work.review-note-required")}</p>
        ) : null}
        {send.isError ? <Alert tone={alertTone.danger}>{messageOf(send.error)}</Alert> : null}
      </div>
    </Window>
  );
}

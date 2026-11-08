import { t } from "@bugaboo/messages";
import { Alert, alertTone, IssueId, partOf, part, type LongPreviewGroup } from "@bugaboo/ui";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { messageOf } from "../../shared/errors.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { skipReasonText, useUndoPreview, type UndoSkip } from "../runs/index.ts";
import { runsOf, type AttemptRun, type Work } from "./agent-work.ts";

/** What stays of one skipped field: the issue is the group, the field and why are the line. */
function groupsOf(skipped: readonly UndoSkip[]): readonly LongPreviewGroup[] {
  return skipped.map((skip, at) => ({
    key: [skip.issue?.id ?? "", skip.field, at].join("/"),
    skipped: true,
    heading:
      skip.issue === undefined || skip.issue === null ? null : (
        <IssueId id={skip.issue.readable_id} />
      ),
    lines: [
      {
        key: "stay",
        content: t("agent-work.throw-away-stay", {
          field: fieldNameOf({ code: skip.field, name: skip.name }),
          reason: skipReasonText(skip),
        }),
      },
    ],
  }));
}

/** What undoing the run of one attempt would do now: how many changes go back and which fields stay. */
function RunPreview({ attempt }: { readonly attempt: AttemptRun }) {
  const read = useUndoPreview(attempt.run, true);
  const preview = read.data;
  return (
    <div {...partOf(part.agentWorkSection)}>
      <h4 {...partOf(part.agentWorkLabel)}>
        {t("agent-work.attempt", { number: attempt.number })}
      </h4>
      {read.isError ? <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert> : null}
      {preview === undefined ? (
        read.isError ? null : (
          <p {...partOf(part.muted)}>{t("agent-work.throw-away-reading")}</p>
        )
      ) : (
        <AppLongPreview
          summary={t("agent-work.throw-away-summary", {
            back: preview.goes_back,
            stay: preview.skipped_count,
          })}
          groups={groupsOf(preview.skipped)}
          label={t("agent-work.throw-away-list")}
        />
      )}
    </div>
  );
}

/**
 * What *Throw away* would do (AI-35): for the run of every attempt, newest first, what goes back and
 * what is skipped, in the one long preview of the app (as *Undo the run* shows it), and the
 * sentence that a branch or anything else outside Bugaboo stays as it is.
 */
export function ThrowAwayPreview({ work }: { readonly work: Work }) {
  return (
    <div {...partOf(part.agentWorkSection)}>
      {runsOf(work).map((attempt) => (
        <RunPreview key={attempt.run} attempt={attempt} />
      ))}
      <p {...partOf(part.muted)}>{t("agent-work.throw-away-outside")}</p>
    </div>
  );
}

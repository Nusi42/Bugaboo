import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import type { BulkApplied } from "./bulk.ts";
import { RunProgress, SkippedList } from "./BulkResultView.tsx";
import { changedIssues } from "./undo.ts";

/** What the last change did: it failed, it goes on in the background, or it is done and what it skipped. */
export function BulkOutcome({ applied }: { readonly applied: BulkApplied }) {
  const { result, failed } = applied;
  if (failed !== null) {
    return <Alert tone={alertTone.danger}>{failed}</Alert>;
  }
  if (result?.run !== undefined && result.run !== null) {
    return <RunProgress run={result.run.id} />;
  }
  const outcomes = result?.outcomes ?? [];
  const changed = result === null ? 0 : changedIssues(result).length;
  return (
    <output {...partOf(part.bulkProgress)}>
      <Alert tone={alertTone.success}>
        {t("bulk.result-done", { changed, skipped: outcomes.length - changed })}
      </Alert>
      <SkippedList
        skipped={outcomes.flatMap((outcome) =>
          outcome.skipped === undefined || outcome.skipped === null
            ? []
            : [{ issue: outcome.issue ?? null, reason: outcome.skipped }],
        )}
      />
    </output>
  );
}

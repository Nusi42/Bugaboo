import { t } from "@bugaboo/messages";
import { Alert, alertTone, FieldValue, partOf, part } from "@bugaboo/ui";
import { useEffect } from "react";

import { messageOf } from "../../shared/errors.ts";
import { isRunning, skipText, useBulkRun, type BulkRun } from "./bulk.ts";
import { progressLook } from "./progress-look.ts";
import { refreshIssues } from "../../shared/query-keys.ts";

/** The issues that were left alone and why, one line each. */
export function SkippedList({ skipped }: { readonly skipped: BulkRun["skipped"] }) {
  if (skipped.length === 0) {
    return null;
  }
  const lines: readonly { readonly key: string; readonly text: string }[] = skipped.map((each) => ({
    key: [each.issue?.id ?? "", each.reason].join(":"),
    text: t("bulk.skipped-line", {
      issue: each.issue?.key ?? t("bulk.issue-unknown"),
      reason: skipText(each.reason),
    }),
  }));
  return (
    <ul {...partOf(part.list)} aria-label={t("bulk.skipped-list")}>
      {lines.map((line) => (
        <li key={line.key} {...partOf(part.itemText)}>
          {line.text}
        </li>
      ))}
    </ul>
  );
}

/** How far a change in the background is, read every second until it is over. */
export function RunProgress({ run }: { readonly run: string }) {
  const read = useBulkRun(run);
  const data = read.data;
  const over = data !== undefined && !isRunning(data);
  useEffect(() => {
    if (over) {
      refreshIssues();
    }
  }, [over]);
  if (read.isError) {
    return <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert>;
  }
  if (data === undefined) {
    return null;
  }
  if (data.status === "failed" || data.status === "stopped") {
    return <Alert tone={alertTone.danger}>{t("bulk.stopped")}</Alert>;
  }
  if (over) {
    return (
      <div {...partOf(part.bulkProgress)}>
        <Alert tone={alertTone.success}>
          {t("bulk.result-done", { changed: data.changed, skipped: data.skipped_count })}
        </Alert>
        <SkippedList skipped={data.skipped} />
      </div>
    );
  }
  return (
    <output {...partOf(part.bulkProgress)} aria-label={t("bulk.progress-label")}>
      <FieldValue
        look={progressLook}
        text={t("bulk.progress", { done: data.done, total: data.total })}
        fraction={data.total === 0 ? 0 : data.done / data.total}
      />
    </output>
  );
}

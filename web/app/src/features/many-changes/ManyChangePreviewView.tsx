import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part, type LongPreviewGroup } from "@bugaboo/ui";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { changeValueText } from "../../shared/change-value.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { skipText, type ManyChangePreview, type ManyChangeRow } from "./many-changes.ts";

export interface ManyChangePreviewViewProps {
  readonly preview: ManyChangePreview;
  /** What happens to each issue that changes when the row has nothing more to say: "Is deleted". */
  readonly plain: string;
}

/** The lines under an issue that will change: what changes in it, or the one thing that happens. */
function changeLines(row: ManyChangeRow, plain: string): LongPreviewGroup["lines"] {
  const detail = row.detail;
  if (detail?.kind === "changes") {
    return detail.fields.map((change) => ({
      key: change.field,
      content: t("many-changes.line-changes", {
        field: fieldNameOf({ code: change.field, name: change.name }),
        after: changeValueText(change.field, change.after) ?? t("common.nothing"),
      }),
    }));
  }
  if (detail?.kind === "relation") {
    return [
      {
        key: "relation",
        content: t("many-changes.line-relation", { name: detail.name, issue: detail.issue.key }),
      },
    ];
  }
  return [{ key: "plain", content: plain }];
}

/** One issue of the preview: its readable ID and what happens to it, or why it is left alone. */
function groupOf(row: ManyChangeRow, plain: string): LongPreviewGroup {
  const skipped = row.skipped !== undefined && row.skipped !== null;
  return {
    key: row.address,
    heading: <strong>{row.issue?.key ?? t("many-changes.issue-unknown")}</strong>,
    lines: skipped ? [{ key: "skipped", content: skipText(row.skipped) }] : changeLines(row, plain),
    skipped,
  };
}

/**
 * What the change would do to each ticked issue (*Show, then do*), as the one long preview of the
 * app: the sentence on top, the issues below, the ones that are left alone quieter.
 */
export function ManyChangePreviewView({ preview, plain }: ManyChangePreviewViewProps) {
  return (
    <div {...partOf(part.stack)}>
      <AppLongPreview
        summary={t("preview.summary", {
          changed: preview.summary.will_change,
          skipped: preview.summary.skipped,
        })}
        label={t("preview.label")}
        nothing={t("preview.nothing")}
        groups={preview.rows.map((row) => groupOf(row, plain))}
      />
      {preview.runs_in_background ? (
        <Alert tone={alertTone.info}>{t("many-changes.background-note")}</Alert>
      ) : null}
    </div>
  );
}

import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Icon, iconName, partOf, part, type LongPreviewGroup } from "@bugaboo/ui";
import type { Wording } from "@bugaboo/wasm";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { wordedSideOf } from "../../shared/change-sentence.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { renderSaid } from "../../shared/said.ts";
import { WithWording } from "../../shared/with-wording.tsx";
import { skipText, type ManyChangePreview, type ManyChangeRow } from "./many-changes.ts";

export interface ManyChangePreviewViewProps {
  readonly preview: ManyChangePreview;
  /** What happens to each issue that changes when the row has nothing more to say: "Is deleted". */
  readonly plain: string;
}

/** What a field will be after the change, in words; *nothing* for no value. */
function afterText(wording: Wording, change: Schema<"FieldChangeBody">): string {
  const side = wordedSideOf(change.after);
  const said = side === null ? null : wording.sideWords(change.field, side);
  return said === null ? t("common.nothing") : renderSaid(said);
}

/** The lines under an issue that will change: what changes in it, or the one thing that happens. */
function changeLines(
  row: ManyChangeRow,
  plain: string,
  wording: Wording,
): LongPreviewGroup["lines"] {
  const detail = row.detail;
  if (detail?.kind === "changes") {
    return [
      ...detail.fields.map((change) => ({
        key: change.field,
        content: t("many-changes.line-changes", {
          field: fieldNameOf({ code: change.field, name: change.name }),
          after: afterText(wording, change),
        }),
      })),
      // What the person should know besides what changes: the issue keeps a type the project does
      // not use (M3-F124).
      ...detail.notes.map((note) => ({
        key: `note-${note.issue_type}`,
        content: t("many-changes.note-type-not-used", {
          type: note.issue_type,
          project: note.project,
        }),
      })),
    ];
  }
  if (detail?.kind === "relation") {
    return [
      {
        key: "relation",
        content: t("many-changes.line-relation", {
          name: detail.name,
          issue: detail.issue.readable_id,
        }),
      },
    ];
  }
  return [{ key: "plain", content: plain }];
}

/** One issue of the preview: its readable ID and what happens to it, or why it is left alone. */
function groupOf(row: ManyChangeRow, plain: string, wording: Wording): LongPreviewGroup {
  const skipped = row.skipped !== undefined && row.skipped !== null;
  return {
    key: row.address,
    heading: (
      <>
        <strong>{row.issue?.readable_id ?? t("many-changes.issue-unknown")}</strong>
        {skipped ? (
          <span {...partOf(part.row)}>
            <Icon name={iconName.close} />
            {t("many-changes.skipped-word")}
          </span>
        ) : null}
      </>
    ),
    lines: skipped
      ? [{ key: "skipped", content: skipText(row.skipped) }]
      : changeLines(row, plain, wording),
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
      <WithWording>
        {(wording) => (
          <AppLongPreview
            summary={t("preview.summary", {
              changed: preview.summary.will_change,
              skipped: preview.summary.skipped,
            })}
            label={t("preview.label")}
            nothing={t("preview.nothing")}
            groups={preview.rows.map((row) => groupOf(row, plain, wording))}
          />
        )}
      </WithWording>
      {preview.runs_in_background ? (
        <Alert tone={alertTone.info}>{t("many-changes.background-note")}</Alert>
      ) : null}
    </div>
  );
}

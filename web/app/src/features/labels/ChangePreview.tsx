import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import { listText } from "../../shared/list-text.ts";
import { useChangePreview, type ChangeAsked, type SortingKind } from "./change-preview.ts";

/** The names a tag or label list reads as, or a word for none. */
function namesOf(names: readonly string[]): string {
  return names.length === 0 ? t("labels.preview-no-names") : listText(names);
}

/** What a rename or a combination would do to the issues the person sees, grouped by issue. */
export function ChangePreviewView({ preview }: { readonly preview: Schema<"ChangePreviewBody"> }) {
  return (
    <div {...partOf(part.stack)}>
      <AppLongPreview
        summary={t("issues.preview-summary", { changed: preview.summary.will_change, skipped: 0 })}
        label={t("issues.preview-label")}
        nothing={t("issues.preview-nothing")}
        groups={preview.issues.map((each) => ({
          key: each.issue.id,
          heading: <strong>{each.issue.key}</strong>,
          lines: [
            {
              key: "names",
              content: t("labels.preview-line", {
                before: namesOf(each.before),
                after: namesOf(each.after),
              }),
            },
          ],
        }))}
      />
      {preview.truncated ? <p {...partOf(part.muted)}>{t("labels.preview-truncated")}</p> : null}
    </div>
  );
}

export interface ShowChangesProps {
  readonly kind: SortingKind;
  readonly id: string;
  /** What the preview is asked about; nothing when there is nothing to ask yet. */
  readonly asked: ChangeAsked | null;
  /** The name of the button, which says what is previewed. */
  readonly button: string;
}

/** *Show what changes*: asks for the preview of a rename or a combination and shows the answer. */
export function ShowChanges({ kind, id, asked, button }: ShowChangesProps) {
  const { preview, pending, error, ask } = useChangePreview(kind);
  return (
    <div {...partOf(part.stack)}>
      <div {...partOf(part.row)}>
        <Button
          isDisabled={asked === null}
          isPending={pending}
          onPress={() => {
            if (asked !== null) {
              ask(id, asked);
            }
          }}
        >
          {button}
        </Button>
      </div>
      {error === null ? null : <Alert tone={alertTone.danger}>{error}</Alert>}
      {preview === null ? null : <ChangePreviewView preview={preview} />}
    </div>
  );
}

import { t } from "@bugaboo/messages";
import { Alert, alertTone, ConfirmDialog, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { personName } from "../../shared/person.ts";
import {
  refreshRun,
  skipLineText,
  useUndoPreview,
  useUndoRun,
  type Run,
  type UndoPreview,
  type UndoSkip,
} from "./runs.ts";

/** What tells the skipped fields apart: the issue, the field and the place in the list. */
function skipKey(skip: UndoSkip, index: number): string {
  return [skip.issue?.id ?? "", skip.field, index].join("/");
}

/** What the question says: how many go back, and the fields that stay with their reasons. */
function QuestionText({ preview }: { readonly preview: UndoPreview }) {
  const named = preview.skipped.length;
  return (
    <div {...partOf(part.stack)}>
      <div>{t("runs.goes-back", { count: preview.goes_back })}</div>
      {preview.skipped_count === 0 ? null : (
        <div>
          <strong>{t("runs.skipped-title", { count: preview.skipped_count })}</strong>
          <ul {...partOf(part.list)}>
            {preview.skipped.map((skip, index) => (
              <li key={skipKey(skip, index)}>{skipLineText(skip)}</li>
            ))}
          </ul>
          {preview.skipped_count > named ? (
            <div>{t("runs.skipped-more", { count: preview.skipped_count - named })}</div>
          ) : null}
        </div>
      )}
      <div {...partOf(part.muted)}>{t("runs.dialog-note")}</div>
    </div>
  );
}

/** The question of *Undo the run*: who it is about, and the app they used. */
function questionTitle(found: Run): string {
  const person = personName(found.person);
  return found.app === undefined || found.app === null || found.app === ""
    ? t("runs.question-plain", { person })
    : t("runs.question", { person, app: found.app });
}

/** *Undo the run*: one button that opens the question; the answer is said in one line. */
export function UndoRun({ run, found }: { readonly run: string; readonly found: Run }) {
  const over = found.status !== "running" && found.status !== "queued";
  const open = over && (found.undone === undefined || found.undone === null);
  const preview = useUndoPreview(run, open);
  const undo = useUndoRun();
  const done = undo.data;
  if (done !== undefined && !done.changed_since_preview) {
    return (
      <output>
        {t("runs.undone-message", { back: done.goes_back, skipped: done.skipped_count })}
      </output>
    );
  }
  if (!open) {
    return null;
  }
  const shown = done ?? preview.data;
  return (
    <div {...partOf(part.stack)}>
      {preview.isError ? <Alert tone={alertTone.danger}>{messageOf(preview.error)}</Alert> : null}
      {undo.isError ? <Alert tone={alertTone.danger}>{messageOf(undo.error)}</Alert> : null}
      {done?.changed_since_preview === true ? (
        <Alert tone={alertTone.warning}>{t("runs.changed-since-preview")}</Alert>
      ) : null}
      {shown === undefined ? null : (
        <div>
          <ConfirmDialog
            triggerLabel={t("runs.undo")}
            title={questionTitle(found)}
            text={<QuestionText preview={shown} />}
            confirmLabel={t("runs.confirm", { count: shown.goes_back })}
            cancelLabel={t("common.cancel")}
            confirmDisabled={shown.goes_back === 0}
            onConfirm={() => {
              undo.mutate(
                { params: { path: { run } }, body: { preview_version: shown.preview_version } },
                { onSuccess: refreshRun },
              );
            }}
          />
        </div>
      )}
    </div>
  );
}

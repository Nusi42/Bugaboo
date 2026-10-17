import type { EditWhat } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { codeOf, messageOf } from "../errors.ts";
import { personName } from "../person.ts";
import type { EditedKind } from "./edited-thing.ts";
import { useEditingNow } from "./use-editing-now.ts";

export interface RemoveEditableProps {
  /** The thing that is taken away: what its edit lock is about. */
  readonly what: EditWhat;
  /** What the thing is, as a word in the sentences. */
  readonly kind: EditedKind;
  /** Takes it away. */
  readonly onConfirm: () => void;
  /** Why taking it away did not work, if it did not. */
  readonly error: unknown;
}

/**
 * *Use the inherited … again*: takes a layout or a workflow of a place away, after a question.
 * While somebody else is editing it the server refuses, and the page says who it is (as the person
 * may see them).
 */
export function RemoveEditable({ what, kind, onConfirm, error }: RemoveEditableProps) {
  const busy = codeOf(error) === "edit.in_progress";
  const editing = useEditingNow(what, busy);
  const word = t(kind);
  return (
    <>
      <ConfirmDialog
        triggerLabel={t("editing.use-inherited", { kind: word })}
        triggerName={t("editing.use-inherited", { kind: word })}
        title={t("editing.use-inherited-title", { kind: word })}
        text={t("editing.use-inherited-text", { kind: word })}
        confirmLabel={t("editing.use-inherited", { kind: word })}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={onConfirm}
      />
      {error === null || error === undefined ? null : (
        <Alert tone={alertTone.danger}>
          {busy && editing.data !== null && editing.data !== undefined
            ? t("editing.delete-held", { name: personName(editing.data.holder), kind: word })
            : messageOf(error)}
        </Alert>
      )}
    </>
  );
}

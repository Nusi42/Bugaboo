import type { EditWhat } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, buttonLook, ConfirmDialog } from "@bugaboo/ui";

import { codeOf, messageOf } from "../errors.ts";
import { personName } from "../person.ts";
import { useEditingNow } from "./use-editing-now.ts";

/** The words of the question and the notes, which each kind of thing says in its own way. */
export interface RemoveWords {
  /** The button, and the answer *yes* of the question. */
  readonly button: string;
  readonly title: string;
  readonly text: string;
  /** Says that somebody is editing it, so it cannot be taken away yet. */
  readonly held: (name: string) => string;
}

export interface RemoveEditableProps {
  /** The thing that is taken away: what its edit lock is about. */
  readonly what: EditWhat;
  readonly words: RemoveWords;
  /** Takes it away. */
  readonly onConfirm: () => void;
  /** Why taking it away did not work, if it did not. */
  readonly error: unknown;
}

/**
 * Takes a layout or a workflow of a place away so the place follows its parent again, after a
 * question. While somebody else is editing it the server refuses, and the page says who it is (as
 * the person may see them). The words are the caller's: each kind says what it does in its own way.
 */
export function RemoveEditable({ what, words, onConfirm, error }: RemoveEditableProps) {
  const busy = codeOf(error) === "edit.in_progress";
  const editing = useEditingNow(what, busy);
  return (
    <>
      <ConfirmDialog
        triggerLabel={words.button}
        triggerName={words.button}
        title={words.title}
        text={words.text}
        confirmLabel={words.button}
        cancelLabel={t("common.cancel")}
        confirmLook={buttonLook.danger}
        onConfirm={onConfirm}
      />
      {error === null || error === undefined ? null : (
        <Alert tone={alertTone.danger}>
          {busy && editing.data !== null && editing.data !== undefined
            ? words.held(personName(editing.data.holder))
            : messageOf(error)}
        </Alert>
      )}
    </>
  );
}

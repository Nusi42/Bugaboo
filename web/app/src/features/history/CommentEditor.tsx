import { t } from "@bugaboo/messages";
import { alertTone, Alert, Button, partOf, part } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

import { editedThing } from "../../shared/editing/edited-thing.ts";
import { HeldNote } from "../../shared/editing/HeldNote.tsx";
import { messageOf } from "../../shared/errors.ts";
import { CommentEditForm } from "./CommentEditForm.tsx";
import { useCommentLock } from "./use-comment-lock.ts";

interface EditorProps {
  readonly comment: string;
  readonly version: number;
  /** The text in the box; the screen keeps it, so it survives taking the lock again. */
  readonly draft: string;
  readonly onDraft: (text: string) => void;
  /** The comment as it is read, shown while there is no form. */
  readonly reading: ReactNode;
  readonly onClose: () => void;
  /** Takes the lock again, with the text as it is. */
  readonly onRetry: () => void;
}

/** One go at the lock on the comment: the form once it is held, else the comment with the note that says why not. */
function LockedEditor(props: EditorProps) {
  const lock = useCommentLock(props.comment);
  const close = <Button onPress={props.onClose}>{t("common.close")}</Button>;
  if (lock.status === "mine") {
    return (
      <CommentEditForm
        comment={props.comment}
        version={props.version}
        draft={props.draft}
        onDraft={props.onDraft}
        token={lock.token}
        onClose={props.onClose}
        onRetry={props.onRetry}
      />
    );
  }
  if (lock.status === "held") {
    return (
      <>
        {props.reading}
        <HeldNote held={lock.held} thing={editedThing.comment}>
          {close}
        </HeldNote>
      </>
    );
  }
  if (lock.status === "failed") {
    return (
      <>
        {props.reading}
        <Alert tone={alertTone.danger}>
          <div {...partOf(part.issueNote)}>
            <span>{messageOf(lock.error)}</span>
            {close}
          </div>
        </Alert>
      </>
    );
  }
  return props.reading;
}

/**
 * Edits the text of a comment (ISS-05, ADR-0019): under the lock, saved with its token and the
 * version the editor started from; every earlier text is kept by the server. While the form is
 * not open (the lock is somebody else's, or could not be taken) the comment stays readable, with
 * the note that says why and *Close*. When the lock ran out or was taken over, *Try again* takes
 * it again and the typed text stays.
 */
export function CommentEditor(props: {
  readonly comment: string;
  readonly version: number;
  readonly text: string;
  readonly reading: ReactNode;
  readonly onClose: () => void;
}) {
  const [attempt, setAttempt] = useState(0);
  const [draft, setDraft] = useState(props.text);
  return (
    <LockedEditor
      key={attempt}
      comment={props.comment}
      version={props.version}
      draft={draft}
      onDraft={setDraft}
      reading={props.reading}
      onClose={props.onClose}
      onRetry={() => {
        setAttempt(attempt + 1);
      }}
    />
  );
}

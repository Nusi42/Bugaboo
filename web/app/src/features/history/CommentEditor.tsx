import { t } from "@bugaboo/messages";
import { Button, partOf, part } from "@bugaboo/ui";
import type { ReactNode } from "react";

import { useAddFiles } from "../files/index.ts";
import { DraftOffered } from "../../shared/editing/DraftOffered.tsx";
import { editedThing } from "../../shared/editing/edited-thing.ts";
import { HeldEditNotice } from "../../shared/editing/HeldEditNotice.tsx";
import { useNow } from "../../shared/use-now.ts";
import { CommentBox, KeptText } from "./CommentBox.tsx";
import { useCommentText, type CommentSource } from "./use-comment-text.ts";

export interface CommentEditorProps extends CommentSource {
  /** The issue the comment is under, by its readable ID: files pasted into the box go to it. */
  readonly issue: string;
  /** The comment as it is read, shown while someone else holds the lock. */
  readonly reading: ReactNode;
  readonly onClose: () => void;
}

/**
 * Edits the text of a comment (ISS-05, ADR-0019) the way the description is edited: under the
 * lock, with the drafts of the browser and of the server, and a draft that was kept is offered
 * when the editor opens. While someone else holds the lock the comment stays readable, with the
 * note that says who; what the person typed stays visible and is kept as a draft, and *Close*
 * leaves it there. When the lock ran out or was taken over, *Try again* takes it again.
 */
export function CommentEditor({ issue, reading, onClose, ...source }: CommentEditorProps) {
  const now = useNow();
  const addFiles = useAddFiles(issue, source.comment);
  const long = useCommentText(source, onClose);
  const { phase, offered } = long;
  return (
    <>
      {offered === null ? null : (
        <DraftOffered
          found={(when) => t("editing.draft-found", { thing: t(editedThing.comment), when })}
          savedAt={offered.savedAt}
          now={now}
          onContinue={long.continueDraft}
          onDrop={long.dropDraft}
        />
      )}
      {phase.kind === "held" ? (
        <>
          {reading}
          <HeldEditNotice held={phase.lock} thing={editedThing.comment}>
            <Button onPress={onClose}>{t("common.close")}</Button>
          </HeldEditNotice>
          {long.text === source.text ? null : <KeptText text={long.text} />}
          <p {...partOf(part.muted)}>{t("editing.text-kept-on-close")}</p>
        </>
      ) : (
        <CommentBox long={long} onClose={onClose} onFiles={addFiles} />
      )}
    </>
  );
}

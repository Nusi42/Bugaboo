import type { Schema } from "@bugaboo/api-client";
import { MarkdownView } from "@bugaboo/ui";

import { versionOf } from "../../shared/version.ts";
import { CommentEditor } from "./CommentEditor.tsx";

/** The text of a comment: as it reads, or in the editor while its writer changes it. */
export function CommentText({
  entry,
  editing,
  onClose,
}: {
  readonly entry: Schema<"HistoryEntryItem">;
  readonly editing: boolean;
  readonly onClose: () => void;
}) {
  const id = entry.comment;
  const reading =
    entry.html !== undefined && entry.html !== null ? <MarkdownView html={entry.html} /> : null;
  if (editing && id !== undefined && id !== null) {
    return (
      <CommentEditor
        comment={id}
        version={versionOf(entry)}
        text={entry.text ?? ""}
        reading={reading}
        onClose={onClose}
      />
    );
  }
  return reading;
}

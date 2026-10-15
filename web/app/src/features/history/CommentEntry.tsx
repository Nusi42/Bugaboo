import { firstVersion, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { MarkdownView, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { CommentActions } from "./CommentActions.tsx";
import { CommentEditor } from "./CommentEditor.tsx";
import { CommentVersions } from "./CommentVersions.tsx";
import { whoOf } from "./history.ts";

/** One comment of the history: who wrote it and when, its text, and what the reader may do. */
export function CommentEntry({ entry }: { readonly entry: Schema<"HistoryEntryItem"> }) {
  const [editing, setEditing] = useState(false);
  const [versions, setVersions] = useState(false);
  const id = entry.comment;
  const edited = entry.edited_at !== undefined && entry.edited_at !== null;
  return (
    <article {...partOf(part.historyEntry, { variants: ["comment"] })}>
      <span {...partOf(part.historyWhen)}>
        {t("history.by", { who: whoOf(entry), when: formatDateTime(entry.at) })}
      </span>
      {edited ? <span {...partOf(part.historyWhen)}>{t("comments.edited")}</span> : null}
      {editing && id !== undefined && id !== null ? (
        <CommentEditor
          comment={id}
          version={entry.version ?? firstVersion}
          text={entry.text ?? ""}
          onClose={() => {
            setEditing(false);
          }}
        />
      ) : (
        entry.html !== undefined && entry.html !== null && <MarkdownView html={entry.html} />
      )}
      <CommentActions
        comment={entry}
        onEdit={() => {
          setEditing(true);
        }}
        onVersions={() => {
          setVersions(!versions);
        }}
        versionsShown={versions}
      />
      {versions && id !== undefined && id !== null ? <CommentVersions comment={id} /> : null}
    </article>
  );
}

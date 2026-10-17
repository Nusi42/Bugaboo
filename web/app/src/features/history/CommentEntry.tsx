import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { MarkdownView, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { versionOf } from "../../shared/version.ts";
import { CommentActions } from "./CommentActions.tsx";
import { CommentEditor } from "./CommentEditor.tsx";
import { CommentVersions } from "./CommentVersions.tsx";
import { useScrollTo, whoOf } from "./history.ts";

interface CommentEntryProps {
  readonly entry: Schema<"HistoryEntryItem">;
  /** *Open* of a notification came here. */
  readonly highlighted?: boolean;
}

/** Who wrote a comment and when, and whether it was edited. */
function CommentHead({ entry }: { readonly entry: Schema<"HistoryEntryItem"> }) {
  const edited = entry.edited_at !== undefined && entry.edited_at !== null;
  return (
    <>
      <span {...partOf(part.historyWhen)}>
        {t("history.by", { who: whoOf(entry), when: formatDateTime(entry.at) })}
      </span>
      {edited ? <span {...partOf(part.historyWhen)}>{t("comments.edited")}</span> : null}
    </>
  );
}

/** One comment of the history: who wrote it and when, its text, and what the reader may do. */
export function CommentEntry({ entry, highlighted = false }: CommentEntryProps) {
  const shown = useScrollTo<HTMLElement>(highlighted);
  const [editing, setEditing] = useState(false);
  const [versions, setVersions] = useState(false);
  const id = entry.comment;
  const reading =
    entry.html !== undefined && entry.html !== null ? <MarkdownView html={entry.html} /> : null;
  return (
    <article
      ref={shown}
      {...partOf(part.historyEntry, {
        variants: ["comment"],
        states: [highlighted && "highlighted"],
      })}
    >
      <CommentHead entry={entry} />
      {editing && id !== undefined && id !== null ? (
        <CommentEditor
          comment={id}
          version={versionOf(entry)}
          text={entry.text ?? ""}
          reading={reading}
          onClose={() => {
            setEditing(false);
          }}
        />
      ) : (
        reading
      )}
      <CommentActions
        comment={entry}
        editing={editing}
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

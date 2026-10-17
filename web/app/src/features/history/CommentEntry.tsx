import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { CommentFiles } from "../files/index.ts";
import { CommentActions } from "./CommentActions.tsx";
import { CommentText } from "./CommentText.tsx";
import { CommentVersions } from "./CommentVersions.tsx";
import { whoOf } from "./history.ts";

/** One comment of the history: who wrote it and when, its text, and what the reader may do. */
export function CommentEntry({
  issue,
  entry,
}: {
  readonly issue: string;
  readonly entry: Schema<"HistoryEntryItem">;
}) {
  const [editing, setEditing] = useState(false);
  const [versions, setVersions] = useState(false);
  const id = entry.comment;
  const edited = entry.edited_at !== undefined && entry.edited_at !== null;
  const known = id !== undefined && id !== null;
  return (
    <article {...partOf(part.historyEntry, { variants: ["comment"] })}>
      <span {...partOf(part.historyWhen)}>
        {t("history.by", { who: whoOf(entry), when: formatDateTime(entry.at) })}
      </span>
      {edited ? <span {...partOf(part.historyWhen)}>{t("comments.edited")}</span> : null}
      <CommentText
        entry={entry}
        editing={editing}
        onClose={() => {
          setEditing(false);
        }}
      />
      {known ? <CommentFiles issue={issue} comment={id} mayAdd={entry.may_edit === true} /> : null}
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
      {versions && known ? <CommentVersions comment={id} /> : null}
    </article>
  );
}

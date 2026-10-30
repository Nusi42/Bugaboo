import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { CommentFiles } from "../files/index.ts";
import { CommentActions } from "./CommentActions.tsx";
import { CommentText } from "./CommentText.tsx";
import { CommentVersions } from "./CommentVersions.tsx";
import { useScrollTo, whoOf } from "./history.ts";

interface CommentEntryProps {
  readonly issue: string;
  readonly entry: Schema<"HistoryEntryItem">;
  /** *Open* of a notification came here. */
  readonly highlighted?: boolean;
}

/** One comment of the history: who wrote it and when, its text, and what the reader may do. */
export function CommentEntry({ issue, entry, highlighted = false }: CommentEntryProps) {
  const shown = useScrollTo<HTMLElement>(highlighted);
  const [editing, setEditing] = useState(false);
  const [versions, setVersions] = useState(false);
  const id = entry.comment;
  const edited = entry.edited_at !== undefined && entry.edited_at !== null;
  const known = id !== undefined && id !== null;
  return (
    <article
      ref={shown}
      {...partOf(part.historyEntry, {
        variants: ["comment"],
        states: [highlighted && "highlighted"],
      })}
    >
      <span {...partOf(part.historyWhen)}>
        {t("history.by", { who: whoOf(entry), when: formatDateTime(entry.at) })}
      </span>
      {edited ? <span {...partOf(part.historyWhen)}>{t("comments.edited")}</span> : null}
      <CommentText
        issue={issue}
        entry={entry}
        editing={editing}
        onClose={() => {
          setEditing(false);
        }}
      />
      {/* Files are added to an own comment while it is edited, not beside its other buttons. */}
      {known ? (
        <CommentFiles issue={issue} comment={id} mayAdd={entry.may_edit === true && editing} />
      ) : null}
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

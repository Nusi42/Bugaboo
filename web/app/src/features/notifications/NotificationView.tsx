import { t } from "@bugaboo/messages";
import {
  Button,
  MarkdownView,
  partOf,
  part,
  longestNameIn,
  SentenceWithName,
  Skeleton,
  type RowActions,
} from "@bugaboo/ui";
import { Suspense, useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { useSwipedRow } from "../../shared/use-row-swipe.tsx";
import { useWording } from "../../shared/use-wording.ts";
import { RunProgress } from "../many-changes/index.ts";
import { NotificationActions } from "./NotificationActions.tsx";
import { fileNamesOf, sentencesOf, type NotificationRow } from "./notifications.ts";

export interface NotificationViewProps {
  readonly row: NotificationRow;
  /** *Mark as read* for this notification. */
  readonly onRead: (id: string) => void;
  /** Reading is on its way. */
  readonly isReading: boolean;
}

/** The text of the comment a notification is about, as the server made it safe for the reader. */
function CommentText({ row }: { readonly row: NotificationRow }) {
  const html = row.entries.find((entry) => entry.html !== undefined)?.html;
  return html === undefined ? null : <MarkdownView html={html} />;
}

/** The result of the run a notification says has ended, shown when the person asks for it. */
function RunResult({ row }: { readonly row: NotificationRow }) {
  const [shown, setShown] = useState(false);
  const run = row.entries.find((entry) => entry.run !== undefined && entry.run !== null)?.run;
  if (run === undefined || run === null) {
    return null;
  }
  return (
    <>
      <div {...partOf(part.notificationActions)}>
        <Button
          onPress={() => {
            setShown(!shown);
          }}
        >
          {shown ? t("notifications.hide-result") : t("notifications.show-result")}
        </Button>
      </div>
      {shown ? (
        <Suspense fallback={null}>
          <RunProgress run={run} />
        </Suspense>
      ) : null}
    </>
  );
}

/** What a swipe on an unread notification brings in, and its *More* holds: *Mark as read* (UX §3.3, §4.11). */
function rowActions(row: NotificationRow, onRead: (id: string) => void): RowActions {
  return row.read
    ? {}
    : {
        end: [
          {
            id: "mark-read",
            label: t("notifications.mark-read"),
            onAction: () => {
              onRead(row.id);
            },
          },
        ],
      };
}

/** What one notification says, once the shared wording is there. */
function Sentences({
  sentences,
  fileNames,
}: {
  readonly sentences: readonly string[];
  /** The files the notification is about: each sentence keeps the name it has in it whole. */
  readonly fileNames: readonly string[];
}) {
  return (
    <ul {...partOf(part.notificationSentences)}>
      {sentences.map((sentence) => (
        <li key={sentence}>
          <SentenceWithName sentence={sentence} name={longestNameIn(sentence, fileNames)} />
        </li>
      ))}
    </ul>
  );
}

/**
 * One notification (UX §4.11): what happened and when, the text of a comment, and *Open*, which
 * goes to the exact comment or change, and *Mark as read*. An unread one is marked by its color
 * and by the word *New*. The words come from the shared wording (`bb-core`), which a skeleton
 * waits for the first time.
 */
export function NotificationView({ row, onRead, isReading }: NotificationViewProps) {
  const loaded = useWording();
  const sentences = loaded.state === "ready" ? sentencesOf(loaded.wording, row) : [];
  const what = sentences.length === 0 ? t("notifications.new") : listText(sentences);
  const actions = rowActions(row, onRead);
  const swipe = useSwipedRow(actions, what);
  return (
    <li {...partOf(part.notification, { states: [!row.read && "unread"] })} {...swipe.rowProps}>
      <div {...partOf(part.notificationHead)}>
        {row.read ? null : (
          <strong {...partOf(part.notificationNew)}>{t("notifications.new")}</strong>
        )}
        <span {...partOf(part.historyWhen)}>{formatDateTime(row.at)}</span>
      </div>
      {loaded.state === "ready" ? (
        <Sentences sentences={sentences} fileNames={fileNamesOf(row)} />
      ) : null}
      {loaded.state === "loading" ? <Skeleton /> : null}
      <CommentText row={row} />
      <NotificationActions
        row={row}
        what={what}
        onRead={onRead}
        isReading={isReading}
        more={swipe.more}
      />
      <RunResult row={row} />
    </li>
  );
}

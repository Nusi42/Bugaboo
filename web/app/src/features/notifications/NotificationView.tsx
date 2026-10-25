import { t } from "@bugaboo/messages";
import {
  Button,
  buttonLook,
  MarkdownView,
  partOf,
  part,
  SentenceWithName,
  Skeleton,
} from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { Suspense, useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { useWording } from "../../shared/use-wording.ts";
import { addressInProject, issueAddress, issueTargetAddress, paths } from "../../shared/paths.ts";
import { RunProgress } from "../many-changes/index.ts";
import {
  failedRenewalProjectOf,
  fileNamesOf,
  sentencesOf,
  targetOf,
  type NotificationRow,
} from "./notifications.ts";

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

/** *Open the project settings* for a renewal of issue IDs that gave up, where the person may know the project. */
function ProjectSettingsButton({ row }: { readonly row: NotificationRow }) {
  const navigate = useNavigate();
  const shortName = failedRenewalProjectOf(row);
  if (shortName === undefined) {
    return null;
  }
  return (
    <Button
      onPress={() => {
        void navigate({ href: addressInProject(paths.projectSettings, shortName) });
      }}
    >
      {t("notifications.open-project-settings")}
    </Button>
  );
}

/** The buttons of a notification: *Open*, the one for a renewal that gave up, and *Mark as read*. */
function NotificationActions({
  row,
  what,
  onRead,
  isReading,
}: NotificationViewProps & { readonly what: string }) {
  const navigate = useNavigate();
  const [key, target] = [row.issue?.readable_id, targetOf(row)];
  return (
    <div {...partOf(part.notificationActions)}>
      {key === undefined ? null : (
        <Button
          aria-label={t("notifications.open-name", { what })}
          onPress={() => {
            void navigate({
              href: target === undefined ? issueAddress(key) : issueTargetAddress(key, target),
            });
          }}
        >
          {t("notifications.open")}
        </Button>
      )}
      <ProjectSettingsButton row={row} />
      {row.read ? null : (
        <Button
          look={buttonLook.quiet}
          isPending={isReading}
          aria-label={t("notifications.mark-read-name", { what })}
          onPress={() => {
            onRead(row.id);
          }}
        >
          {t("notifications.mark-read")}
        </Button>
      )}
    </div>
  );
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
          <SentenceWithName
            sentence={sentence}
            name={fileNames.find((name) => sentence.includes(name))}
          />
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
  return (
    <li {...partOf(part.notification, { states: [!row.read && "unread"] })}>
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
      <NotificationActions row={row} what={what} onRead={onRead} isReading={isReading} />
      <RunResult row={row} />
    </li>
  );
}

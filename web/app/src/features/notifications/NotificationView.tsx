import { t } from "@bugaboo/messages";
import { Button, buttonLook, MarkdownView, partOf, part } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { Suspense, useState } from "react";

import { formatDateTime } from "../../shared/format.ts";
import { listText } from "../../shared/list-text.ts";
import { addressInProject, issueAddress, issueTargetAddress, paths } from "../../shared/paths.ts";
import { RunProgress } from "../many-changes/index.ts";
import {
  failedRenewalProjectOf,
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

/**
 * One notification (UX §4.11): what happened and when, the text of a comment, and *Open*, which
 * goes to the exact comment or change, and *Mark as read*. An unread one is marked by its color
 * and by the word *New*.
 */
export function NotificationView({ row, onRead, isReading }: NotificationViewProps) {
  const navigate = useNavigate();
  const sentences = sentencesOf(row);
  const what = listText(sentences);
  const [key, target] = [row.issue?.key, targetOf(row)];
  return (
    <li {...partOf(part.notification, { states: [!row.read && "unread"] })}>
      <div {...partOf(part.notificationHead)}>
        {row.read ? null : (
          <strong {...partOf(part.notificationNew)}>{t("notifications.new")}</strong>
        )}
        <span {...partOf(part.historyWhen)}>{formatDateTime(row.at)}</span>
      </div>
      <ul {...partOf(part.notificationSentences)}>
        {sentences.map((sentence) => (
          <li key={sentence}>{sentence}</li>
        ))}
      </ul>
      <CommentText row={row} />
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
      <RunResult row={row} />
    </li>
  );
}

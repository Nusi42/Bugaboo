import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import { issueAddress } from "../../shared/paths.ts";
import { NotificationView } from "./NotificationView.tsx";
import { groupsOf, type NotificationRow } from "./notifications.ts";

export interface NotificationGroupsProps {
  readonly rows: readonly NotificationRow[];
  /** *Mark as read* for one notification. */
  readonly onRead: (id: string) => void;
  /** Reading is on its way. */
  readonly isReading: boolean;
}

/**
 * The notifications grouped by issue, newest first (UX §4.11): the issue is the heading of its
 * group, a link to the issue, and the notifications about it are the list below.
 */
export function NotificationGroups({ rows, onRead, isReading }: NotificationGroupsProps) {
  return groupsOf(rows).map((group) => (
    <section
      key={group.key}
      {...partOf(part.notificationGroup)}
      aria-label={t("notifications.group-name", {
        issue: group.issue?.key ?? t("nav.notifications"),
      })}
    >
      {group.issue === null || group.issue === undefined ? null : (
        <h3 {...partOf(part.notificationGroupHead)}>
          <Link href={issueAddress(group.issue.key)}>{group.issue.key}</Link>
          {group.issue.title === undefined || group.issue.title === null ? null : (
            <span>{group.issue.title}</span>
          )}
        </h3>
      )}
      <ul {...partOf(part.list)}>
        {group.rows.map((row) => (
          <NotificationView key={row.id} row={row} onRead={onRead} isReading={isReading} />
        ))}
      </ul>
    </section>
  ));
}

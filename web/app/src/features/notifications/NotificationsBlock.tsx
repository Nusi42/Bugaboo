import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Card, Link } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import { NotificationEntries } from "./NotificationEntries.tsx";

/** What the block lists, and what it says when there is none. */
const unread: Schema<"NotificationTab"> = "unread";
const none: MessageKey = "notifications.start-nothing";

/** How many unread notifications the start page asks for: the newest few. */
const SHOWN = 5;

/**
 * *Notifications* on the start page (UX §4.1, M4-Q7): the newest unread ones, in place of the old
 * *Mentions* block. Everything else is on the page of notifications.
 */
export function NotificationsBlock() {
  return (
    <Card heading={t("notifications.title")}>
      <NotificationEntries filter={unread} newest={SHOWN} nothing={none} />
      <Link href={paths.notifications}>{t("notifications.show-all")}</Link>
    </Card>
  );
}

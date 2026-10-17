import { t } from "@bugaboo/messages";
import { Alert, alertTone, Card, Link, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { paths } from "../../shared/paths.ts";
import { NotificationGroups } from "./NotificationGroups.tsx";
import { rowsOf } from "./notifications.ts";
import { useNotificationList, useReadNotifications } from "./use-notifications.ts";

/** How many unread notifications the start page asks for: the newest few. */
const SHOWN = 5;

/**
 * *Notifications* on the start page (UX §4.1, M4-Q7): the newest unread ones, in place of the old
 * *Mentions* block. Everything else is on the page of notifications.
 */
export function NotificationsBlock() {
  const list = useNotificationList("unread", SHOWN);
  const read = useReadNotifications();
  const rows = rowsOf(itemsOf(list.data?.pages));
  return (
    <Card heading={t("notifications.title")}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {rows.length === 0 && list.isSuccess ? (
        <p {...partOf(part.muted)}>{t("notifications.start-nothing")}</p>
      ) : null}
      <NotificationGroups
        rows={rows}
        isReading={read.isPending}
        onRead={(id) => {
          read.mutate({ body: { ids: [id] } });
        }}
      />
      <Link href={paths.notifications}>{t("notifications.show-all")}</Link>
    </Card>
  );
}

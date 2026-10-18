import { notificationFilterWords, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Button, buttonLook, Card, Tabs } from "@bugaboo/ui";
import { useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { paths } from "../../shared/paths.ts";
import { useUnreadNotifications } from "../../shared/unread-notifications.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotificationEntries } from "./NotificationEntries.tsx";
import { useNewestNotification, useReadNotifications } from "./use-notifications.ts";

/** The name of each filter. */
const filterName = {
  all: "notifications.filter.all",
  unread: "notifications.filter.unread",
  mentions: "notifications.filter.mentions",
} as const satisfies Record<Schema<"NotificationFilter">, MessageKey>;

/**
 * *Notifications* (UX §4.11, NTF-01): what happened on the issues the person follows and where they
 * are named, grouped by issue, newest first, with the filters *All*, *Unread* and *Mentions*, and
 * *Customize* at the top right.
 */
export function NotificationsPage() {
  usePageTitle(t("notifications.title"));
  const navigate = useNavigate();
  const [filter, setFilter] = useState<Schema<"NotificationFilter">>("all");
  const unread = useUnreadNotifications() ?? 0;
  const readAll = useReadNotifications();
  // *Mark all as read* goes up to the newest one on the screen: a later one stays unread.
  const newest = useNewestNotification();
  return (
    <Card
      heading={t("notifications.title")}
      level={1}
      actions={
        <>
          {unread === 0 || newest === undefined ? null : (
            <Button
              look={buttonLook.quiet}
              isPending={readAll.isPending}
              onPress={() => {
                readAll.mutate({ body: { up_to: newest } });
              }}
            >
              {t("notifications.mark-all-read")}
            </Button>
          )}
          <Button
            look={buttonLook.quiet}
            onPress={() => {
              void navigate({ href: paths.notificationSettings });
            }}
          >
            {t("notifications.customize")}
          </Button>
        </>
      }
    >
      <Tabs
        label={t("notifications.filters")}
        tabs={notificationFilterWords.map((each) => ({ id: each, label: t(filterName[each]) }))}
        selected={filter}
        onSelect={setFilter}
      >
        {(each) => <NotificationEntries filter={each} />}
      </Tabs>
    </Card>
  );
}

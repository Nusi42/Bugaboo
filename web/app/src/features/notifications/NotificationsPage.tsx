import { notificationFilterWords, type Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Button, buttonLook, Card, Link, partOf, part } from "@bugaboo/ui";
import { useState } from "react";
import { Tab, TabList, TabPanel, Tabs } from "react-aria-components";

import { paths } from "../../shared/paths.ts";
import { useUnreadNotifications } from "../../shared/unread-notifications.ts";
import { usePageTitle } from "../../shared/use-page.ts";
import { NotificationEntries } from "./NotificationEntries.tsx";
import { useReadNotifications } from "./use-notifications.ts";

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
  const [filter, setFilter] = useState<Schema<"NotificationFilter">>("all");
  const unread = useUnreadNotifications() ?? 0;
  const readAll = useReadNotifications();
  return (
    <Card heading={t("notifications.title")} level={1}>
      <div {...partOf(part.notificationsBar)}>
        {unread === 0 ? null : (
          <Button
            look={buttonLook.quiet}
            isPending={readAll.isPending}
            onPress={() => {
              readAll.mutate({ body: { all: true } });
            }}
          >
            {t("notifications.mark-all-read")}
          </Button>
        )}
        <Link href={paths.notificationSettings}>{t("notifications.customize")}</Link>
      </div>
      <Tabs
        selectedKey={filter}
        onSelectionChange={(key) => {
          setFilter(notificationFilterWords.find((each) => each === key) ?? "all");
        }}
      >
        <TabList aria-label={t("notifications.filters")} {...partOf(part.formTabs)}>
          {notificationFilterWords.map((each) => (
            <Tab key={each} id={each} {...partOf(part.formTab)}>
              {t(filterName[each])}
            </Tab>
          ))}
        </TabList>
        {notificationFilterWords.map((each) => (
          <TabPanel key={each} id={each} {...partOf(part.formPanel)}>
            <NotificationEntries filter={each} />
          </TabPanel>
        ))}
      </Tabs>
    </Card>
  );
}

import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { itemsOf } from "../../shared/paged.ts";
import { NotificationGroups } from "./NotificationGroups.tsx";
import { rowsOf } from "./notifications.ts";
import { useNotificationList, useReadNotifications } from "./use-notifications.ts";

/** What each filter says when it finds nothing. */
const nothingMessage = {
  all: "notifications.nothing-all",
  unread: "notifications.nothing-unread",
  mentions: "notifications.nothing-mentions",
} as const;

/** The notifications of one filter, newest first, grouped by issue, with *Show earlier* under them. */
export function NotificationEntries({ filter }: { readonly filter: Schema<"NotificationFilter"> }) {
  const list = useNotificationList(filter);
  const read = useReadNotifications();
  const rows = rowsOf(itemsOf(list.data?.pages));
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {read.isError ? <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert> : null}
      {rows.length === 0 && list.isSuccess && !list.hasNextPage ? (
        <p {...partOf(part.muted)}>{t(nothingMessage[filter])}</p>
      ) : null}
      <NotificationGroups
        rows={rows}
        readingId={read.readingId}
        onRead={(id) => {
          read.mutate({ body: { ids: [id] } });
        }}
      />
      {list.hasNextPage ? (
        <Button
          isPending={list.isFetchingNextPage}
          onPress={() => {
            void list.fetchNextPage();
          }}
        >
          {t("notifications.more")}
        </Button>
      ) : null}
    </div>
  );
}

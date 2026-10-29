import type { Schema } from "@bugaboo/api-client";
import { t, type MessageKey } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part, ShowMoreOf } from "@bugaboo/ui";

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

export interface NotificationEntriesProps {
  readonly filter: Schema<"NotificationTab">;
  /** Only the newest this many, without *Show earlier* (the start page); every page of them without it. */
  readonly newest?: number;
  /** What to say when there are none, where the filter's own words do not fit. */
  readonly nothing?: MessageKey;
}

/**
 * The notifications of one filter, newest first, grouped by issue, with *Show earlier* under them.
 * The page of notifications and the block of the start page are this one list.
 */
export function NotificationEntries({ filter, newest, nothing }: NotificationEntriesProps) {
  const list = useNotificationList(filter, newest);
  const read = useReadNotifications();
  const rows = rowsOf(itemsOf(list.data?.pages));
  return (
    <div {...partOf(part.stack)}>
      {list.isError ? <Alert tone={alertTone.danger}>{messageOf(list.error)}</Alert> : null}
      {read.isError ? <Alert tone={alertTone.danger}>{messageOf(read.error)}</Alert> : null}
      {rows.length === 0 && list.isSuccess && !list.hasNextPage ? (
        <p {...partOf(part.muted)}>{t(nothing ?? nothingMessage[filter])}</p>
      ) : null}
      <NotificationGroups
        rows={rows}
        readingId={read.readingId}
        onRead={(id) => {
          read.mutate({ body: { ids: [id] } });
        }}
      />
      {newest === undefined ? <ShowMoreOf label={t("notifications.more")} list={list} /> : null}
    </div>
  );
}

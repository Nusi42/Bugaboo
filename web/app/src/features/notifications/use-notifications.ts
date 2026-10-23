import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { itemsOf, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys, refreshNotifications } from "../../shared/query-keys.ts";
import { rowsOf } from "./notifications.ts";

/**
 * The notifications of the person, newest first, a page at a time. The key starts with the route,
 * like every key `api.query` makes, so reading notifications again finds it.
 */
export function useNotificationList(
  filter: Schema<"NotificationTab">,
  size: number = pageSize.short,
) {
  return usePaged({
    queryKey: queryKeys.notifications.list(filter, size),
    read: (after) =>
      api.client.GET("/api/v1/notifications", {
        params: { query: { tab: filter, limit: size, ...after } },
      }),
  });
}

/**
 * What *Mark as read* and *Mark all as read* do. `readingId` is the notification that *Mark as read*
 * is saving now, if one is: only its button waits.
 */
export function useReadNotifications() {
  const reading = api.query.useMutation("post", "/api/v1/notifications/read", {
    onSuccess: refreshNotifications,
  });
  const readingId = reading.isPending ? reading.variables.body.ids?.[0] : undefined;
  return { ...reading, readingId };
}

/** The ID of the newest notification the page of notifications shows; nothing before it is read. */
export function useNewestNotification(): string | undefined {
  return rowsOf(itemsOf(useNotificationList("all").data?.pages))[0]?.id;
}

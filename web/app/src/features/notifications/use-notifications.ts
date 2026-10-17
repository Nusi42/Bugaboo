import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { refreshNotifications } from "../../shared/query-keys.ts";

/**
 * The notifications of the person, newest first, a page at a time. The key starts with the route,
 * like every key `api.query` makes, so reading notifications again finds it.
 */
export function useNotificationList(
  filter: Schema<"NotificationFilter">,
  size: number = pageSize.short,
) {
  return usePaged({
    queryKey: ["get", "/api/v1/notifications", "list", filter, size],
    read: (after) =>
      api.client.GET("/api/v1/notifications", {
        params: { query: { filter, limit: size, ...after } },
      }),
  });
}

/** What *Mark as read* and *Mark all as read* do. */
export function useReadNotifications() {
  return api.query.useMutation("post", "/api/v1/notifications/read", {
    onSuccess: refreshNotifications,
  });
}

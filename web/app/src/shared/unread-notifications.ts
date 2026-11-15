import {
  BACKGROUND_HEADER,
  msPerSecond,
  notificationsAtOnceMax,
  watchSeconds,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useQuery } from "@tanstack/react-query";

import { api } from "./api.ts";

/**
 * The number of notifications the person has not read, for the menu. Until live updates arrive it
 * is asked every 30 seconds and when the person comes back to the tab, like the versions of what
 * is on the screen (ADR-0019). The ask is marked as a read nobody asked for, so a tab left open
 * does not keep the session alive (SEC-12).
 */
export function unreadOptions() {
  return api.query.queryOptions(
    "get",
    "/api/v1/notifications/unread",
    { headers: { [BACKGROUND_HEADER]: "1" } },
    {
      refetchInterval: watchSeconds * msPerSecond,
      refetchOnWindowFocus: true,
      staleTime: 0,
    },
  );
}

/** How many notifications the person has not read (nothing until the server has answered), and whether it is read. */
export function useUnreadNotifications(): {
  readonly count: number | undefined;
  readonly reading: boolean;
} {
  const read = useQuery(unreadOptions());
  return { count: read.data?.unread, reading: read.isLoading };
}

/**
 * The number on the menu entry, as text: empty while there is none (or the server has not answered), so the entry
 * keeps the room of the number, and the cap with a plus when the server stopped counting.
 */
export function useUnreadCountText(): string {
  const unread = useUnreadNotifications().count;
  if (unread === undefined || unread === 0) {
    return "";
  }
  return unread >= notificationsAtOnceMax ? widestCount() : String(unread);
}

/** The widest number the menu shows: the cap with a plus, when the server stopped counting. */
export function widestCount(): string {
  return t("notifications.count-capped", { max: notificationsAtOnceMax });
}

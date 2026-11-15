import {
  BACKGROUND_HEADER,
  msPerSecond,
  notificationsAtOnceMax,
  watchSeconds,
} from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useHoldWhileReading } from "@bugaboo/ui";
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

/**
 * How many notifications the person has not read; nothing until the server has answered. What shows the number is held
 * until then (with its `ReadingHold`): the number would narrow or push what stands beside it (UX §6.4).
 */
export function useUnreadNotifications(): number | undefined {
  const read = useQuery(unreadOptions());
  useHoldWhileReading(read.isLoading);
  return read.data?.unread;
}

/**
 * The number on the menu entry, as text: nothing while there is none (or the server has not
 * answered), and the cap with a plus when the server stopped counting.
 */
export function useUnreadCountText(): string | undefined {
  const unread = useUnreadNotifications();
  if (unread === undefined || unread === 0) {
    return undefined;
  }
  return unread >= notificationsAtOnceMax
    ? t("notifications.count-capped", { max: notificationsAtOnceMax })
    : String(unread);
}

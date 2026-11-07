import type { Schema } from "@bugaboo/api-client";

import { formatCalendarDay, formatDateTime } from "../../shared/format.ts";
import type { Region } from "../../shared/region.ts";

/**
 * When a row happened, as the row says it: the date of a time entry, with no time and no zone, when
 * the server sends only the date (the reader may not see the moment); else the moment.
 */
export function whenOf(item: Readonly<Schema<"HistoryEntryItem">>, region?: Region): string {
  return item.day === undefined || item.day === null
    ? formatDateTime(item.at, region)
    : formatCalendarDay(item.day, region);
}

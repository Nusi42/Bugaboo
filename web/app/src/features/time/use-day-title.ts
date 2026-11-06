import { t } from "@bugaboo/messages";
import { useLayout } from "@bugaboo/ui";

import { dayOfMonth, weekdayAndDate, weekdayName } from "../../shared/clock.ts";
import { useRegion } from "../../shared/region.ts";

/** A day as the title of a window says it: its weekday and date, and `Thu 8` where the window is narrow. */
export function useDayTitle(date: string | undefined): string {
  const region = useRegion();
  const phone = useLayout() === "phone";
  if (date === undefined) {
    return "";
  }
  return phone
    ? t("time.sheet.day-heading", { weekday: weekdayName(date, "short"), day: dayOfMonth(date) })
    : weekdayAndDate(date, "short", region);
}

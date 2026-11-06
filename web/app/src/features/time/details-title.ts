import { t } from "@bugaboo/messages";
import { useLayout } from "@bugaboo/ui";

import type { TimeEntry } from "./edits.ts";
import { useDayTitle } from "./use-day-title.ts";
import { useDetailsFocus } from "./use-details-focus.ts";

/**
 * What the details window shows of where it is: the title, which names the issue and the day of the
 * entry while one is changed, and the focus, which goes into the form and back to the list.
 */
export function useDetailsTitle(
  formId: string,
  issue: string,
  picked: TimeEntry | undefined,
): string {
  const day = useDayTitle(picked?.date);
  const narrow = useLayout() === "phone";
  useDetailsFocus(formId, picked?.id);
  return picked === undefined
    ? t("time.details.title", { issue })
    : t(narrow ? "time.details.change-title-narrow" : "time.details.change-title", { issue, day });
}

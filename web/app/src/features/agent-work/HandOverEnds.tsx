import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { personToday } from "../../shared/clock.ts";
import { addDays, formatCalendarDay } from "../../shared/format.ts";
import { currentRegion } from "../../shared/region.ts";
import { useIssueAgentWork } from "./agent-work.ts";
import { wholeNumber } from "./hand-over-draft.ts";

/**
 * When a hand-over made now would run out: the days typed, or the usual days of the organization,
 * counted from the person's today on the calendar of their region.
 */
export function EndsOn({ issue, days }: { readonly issue: string; readonly days: string }) {
  const [now] = useState(() => Date.now());
  const usual = useIssueAgentWork(issue).data?.hand_over_days;
  const counted = wholeNumber(days) ?? usual;
  if (counted === undefined) {
    return null;
  }
  const region = currentRegion();
  const date = formatCalendarDay(addDays(personToday(region, now), counted), region);
  return <p {...partOf(part.muted)}>{t("agent-work.ends", { date })}</p>;
}

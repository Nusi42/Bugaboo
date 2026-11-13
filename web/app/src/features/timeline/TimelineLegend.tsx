import { t } from "@bugaboo/messages";
import { legendLook, LegendItem } from "@bugaboo/ui";

import { IssueLegend } from "../issues/index.ts";
import type { TimelineRow } from "./timeline-data.ts";
import { daysWithoutWorkOf } from "./use-marks.ts";

/**
 * The one Legend of the timeline, below its tools (UX §4.7): the tints and the priority marks of the
 * rows, then the bracket of a goal, the diamond of its due date, the Today line, and the marks of the
 * people on the timeline: *Away* and *Holiday*, or only the neutral pattern where the bars carry
 * nothing else, and *Plan warning*.
 */
export function TimelineLegend({ rows }: { readonly rows: readonly TimelineRow[] }) {
  const days = daysWithoutWorkOf(rows);
  const distinct = days.distinct || !days.neutral;
  return (
    <IssueLegend
      more={
        <>
          <LegendItem look={legendLook.goal}>{t("timeline.legend-goal")}</LegendItem>
          <LegendItem look={legendLook.due}>{t("timeline.legend-due")}</LegendItem>
          <LegendItem look={legendLook.today}>{t("timeline.today")}</LegendItem>
          {distinct ? (
            <>
              <LegendItem look={legendLook.away}>{t("timeline.away")}</LegendItem>
              <LegendItem look={legendLook.holiday}>{t("timeline.legend-holiday")}</LegendItem>
            </>
          ) : null}
          {days.neutral ? (
            <LegendItem look={legendLook.noWork}>{t("timeline.legend-no-work")}</LegendItem>
          ) : null}
          <LegendItem look={legendLook.warning}>{t("timeline.warning-sign")}</LegendItem>
        </>
      }
    />
  );
}

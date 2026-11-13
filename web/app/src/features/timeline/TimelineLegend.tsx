import { t } from "@bugaboo/messages";
import { legendLook, LegendItem } from "@bugaboo/ui";

import { IssueLegend } from "../issues/index.ts";

/**
 * The one Legend of the timeline, below its tools (UX §4.7): the tints and the priority marks of the
 * rows, then the bracket of a goal, the diamond of its due date and the Today line. *Away*,
 * *Holiday* and *Plan warning* join it with the people on the timeline.
 */
export function TimelineLegend() {
  return (
    <IssueLegend
      more={
        <>
          <LegendItem look={legendLook.goal}>{t("timeline.legend-goal")}</LegendItem>
          <LegendItem look={legendLook.due}>{t("timeline.legend-due")}</LegendItem>
          <LegendItem look={legendLook.today}>{t("timeline.today")}</LegendItem>
        </>
      }
    />
  );
}

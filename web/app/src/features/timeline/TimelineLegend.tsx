import { t } from "@bugaboo/messages";
import { Legend, legendLook, LegendItem } from "@bugaboo/ui";

/**
 * The marks of the timeline in a Legend below its tools (UX §4.7): the bracket of a goal, the
 * diamond of its due date and the Today line. The tints and edges of the rows stand in the Legend
 * of the list beside the toolbar; *Away*, *Holiday* and *Plan warning* join this one with the
 * people on the timeline.
 */
export function TimelineLegend() {
  return (
    <Legend label={t("timeline.legend")}>
      <LegendItem look={legendLook.goal}>{t("timeline.legend-goal")}</LegendItem>
      <LegendItem look={legendLook.due}>{t("timeline.legend-due")}</LegendItem>
      <LegendItem look={legendLook.today}>{t("timeline.today")}</LegendItem>
    </Legend>
  );
}

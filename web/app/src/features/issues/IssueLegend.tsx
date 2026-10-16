import { t } from "@bugaboo/messages";
import { Legend, legendLook, LegendItem } from "@bugaboo/ui";

/**
 * The Legend beside the toolbar of the list (UX §4.2): what the tint of a row and its left edge mean.
 * Each swatch is told in words, because a color is never alone (R5).
 */
export function IssueLegend() {
  return (
    <Legend label={t("issues.legend")}>
      <LegendItem look={legendLook.inProgress}>{t("status-group.in-progress")}</LegendItem>
      <LegendItem look={legendLook.done}>{t("status-group.done")}</LegendItem>
      <LegendItem look={legendLook.critical}>{t("priority.critical")}</LegendItem>
      <LegendItem look={legendLook.high}>{t("priority.high")}</LegendItem>
    </Legend>
  );
}

import { t } from "@bugaboo/messages";
import { Legend, legendLook, LegendItem } from "@bugaboo/ui";
import type { ReactNode } from "react";

/**
 * The Legend beside the toolbar of the list (UX §4.2): what the tint of a row and its left edge mean.
 * Each swatch is told in words, because a color is never alone (R5). The timeline adds its own marks
 * (`more`) and is then the one Legend of its page.
 */
export function IssueLegend({ more }: { readonly more?: ReactNode }) {
  return (
    <Legend label={t("issues.legend")}>
      <LegendItem look={legendLook.notStarted}>{t("status-group.not-started")}</LegendItem>
      <LegendItem look={legendLook.inProgress}>{t("status-group.in-progress")}</LegendItem>
      <LegendItem look={legendLook.done}>{t("status-group.done")}</LegendItem>
      <LegendItem look={legendLook.critical}>{t("priority.critical")}</LegendItem>
      <LegendItem look={legendLook.high}>{t("priority.high")}</LegendItem>
      {more}
    </Legend>
  );
}

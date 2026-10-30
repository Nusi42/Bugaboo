import { t } from "@bugaboo/messages";

/** *302 in Website · 13 shown*; while the rows are still on their way only what is found, never "0 shown". */
export function countLine(total: number, scope: string, shown: number): string {
  return shown === 0 && total > 0
    ? t("issues.count-line-loading", { total, scope })
    : t("issues.count-line", { total, scope, shown });
}

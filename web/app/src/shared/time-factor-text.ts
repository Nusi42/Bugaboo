import { t } from "@bugaboo/messages";

import { formatNumber } from "./format.ts";

/** The percent of the estimate that is ×1. */
export const AS_ESTIMATED = 100;

/**
 * A time factor in percent as it is written: `×1.25`, in the reader's *Number format*. The one
 * writer of a time factor: the row of the issue page and the history use it.
 */
export function factorText(percent: number): string {
  return t("issues.time-factor-value", { factor: formatNumber(percent / AS_ESTIMATED) });
}

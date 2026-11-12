import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { minutesText } from "../../shared/own-value-text.ts";
import { AS_ESTIMATED, factorText } from "./time-factor.ts";

export interface TimeFactorSumProps {
  /** *Estimated time* in minutes; none when the reader may not read it or the issue has none. */
  readonly estimate: number | undefined;
  /** The factor typed, in percent; `null` while the window takes a length or holds no factor. */
  readonly percent: number | null;
  /** The length typed, in minutes; `null` while the window takes a factor or holds no length. */
  readonly minutes: number | null;
}

/**
 * The sum under the box, as it is typed (UX §4.4): "Estimate 8h · ×1.25 = 10h" for a factor,
 * "Estimate 8h · takes 10h" for a length. It only says what is typed beside the estimate; the factor
 * that is stored is the one the server works out.
 */
export function TimeFactorSum({ estimate, percent, minutes }: TimeFactorSumProps) {
  if (estimate === undefined) {
    return null;
  }
  const said = minutesText(estimate);
  let sum = t("issues.time-factor-estimate", { estimate: said });
  if (percent !== null) {
    sum = t("issues.time-factor-sum", {
      estimate: said,
      factor: factorText(percent),
      total: minutesText(Math.round((estimate * percent) / AS_ESTIMATED)),
    });
  } else if (minutes !== null) {
    sum = t("issues.time-factor-sum-length", { estimate: said, length: minutesText(minutes) });
  }
  return <p {...partOf(part.muted)}>{sum}</p>;
}

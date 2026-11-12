import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { minutesText } from "../../shared/own-value-text.ts";
import { factorText } from "../../shared/time-factor-text.ts";
import { useWording } from "../../shared/use-wording.ts";

export interface TimeFactorSumProps {
  /** *Estimated time* in minutes; none when the reader may not read it or the issue has none. */
  readonly estimate: number | undefined;
  /** The time factor typed, in percent; `null` while the window takes a length or holds none. */
  readonly percent: number | null;
  /** The length typed, in minutes; `null` while the window takes a time factor or holds none. */
  readonly minutes: number | null;
}

/**
 * The sum under the box, as it is typed (UX §4.4): "Estimate 8h · ×1.25 = 10h", for a time factor
 * and for a length alike. The time factor of a length and the effort of a time factor are worked out
 * by the code the plan counts with (`bb-core`, through the browser module), so the sum says what the
 * plan will count. Only the estimate is said while nothing that makes a sum is typed.
 */
export function TimeFactorSum({ estimate, percent, minutes }: TimeFactorSumProps) {
  const wording = useWording();
  if (estimate === undefined || wording.state !== "ready") {
    return null;
  }
  const said = minutesText(estimate);
  const factor =
    percent ?? (minutes === null ? undefined : wording.wording.factorOfLength(estimate, minutes));
  const effort = factor === undefined ? undefined : wording.wording.factorEffort(estimate, factor);
  return (
    <p {...partOf(part.muted)}>
      {factor === undefined || effort === undefined
        ? t("issues.time-factor-estimate", { estimate: said })
        : t("issues.time-factor-sum", {
            estimate: said,
            factor: factorText(factor),
            total: minutesText(effort),
          })}
    </p>
  );
}

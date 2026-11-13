import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { factorWords, shownPercent, useTimeFactor } from "./time-factor.ts";
import { TimeFactorChange } from "./TimeFactorChange.tsx";

/**
 * *Time factor* under *Estimated time* (PLAN-14, UX §4.4): "×1.25 · Lea's own estimate", "×1.5 · set
 * by Ada Brandt" or "×1 (as estimated)", and *Change*, which opens the window *How long will it take
 * you?*. The person on the bar always sees the row; anybody else as the field level of *Time factor*
 * allows. Whether *Change* is offered the server says (`may_change`), by the rule that also decides a
 * save. An issue assigned to nobody has no row: the factor is the person's.
 */
export function TimeFactorRow({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const factor = useTimeFactor(blocks.readableId).data;
  const percent = factor === undefined ? undefined : shownPercent(factor);
  if (factor === undefined || percent === undefined || factor.person === null) {
    return null;
  }
  return (
    <DetailRow name={t("issue-field.time-factor")}>
      <span {...partOf(part.nameValueWords)}>{factorWords(factor, percent)}</span>
      <TimeFactorChange
        issue={blocks.issue}
        readableId={blocks.readableId}
        mayReadEstimate={blocks.mask.sees("estimated-time")}
      />
    </DetailRow>
  );
}

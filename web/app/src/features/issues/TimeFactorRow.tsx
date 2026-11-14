import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";

import { useMe } from "../../shared/use-me.ts";
import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { factorForPeople, factorWords, shownPercent, useTimeFactor } from "./time-factor.ts";
import { TimeFactorChange } from "./TimeFactorChange.tsx";

/**
 * The time factor the row shows: the answer of the server, or, after a change of the people on the
 * issue and until the answer of the factor comes, what it will say, so the row changes once.
 */
function useShownFactor(blocks: ChangeableRowsProps["blocks"]) {
  const read = useTimeFactor(blocks.readableId).data;
  const reader = { id: useMe()?.id, mayChangeField: blocks.mask.changes("time-factor") };
  return read === undefined
    ? undefined
    : factorForPeople(read, blocks.issue.assigned_to?.[0], reader);
}

/**
 * *Time factor* under *Estimated time* (PLAN-14, UX §4.4): "×1.25 · Lea's own estimate", "×1.5 · set
 * by Ada Brandt" or "×1 (as estimated)", and *Change*, which opens the window *How long will it take
 * you?*. The person on the bar always sees the row; anybody else as the field level of *Time factor*
 * allows. Whether *Change* is offered the server says (`may_change`), by the rule that also decides a
 * save. An issue assigned to nobody says so in the row: the factor is the person's, and the row
 * keeps its place when somebody is assigned, so nothing under it moves (UX §6.4).
 */
export function TimeFactorRow({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const factor = useShownFactor(blocks);
  const percent = factor === undefined ? undefined : shownPercent(factor);
  if (factor === undefined || percent === undefined) {
    return null;
  }
  return (
    <DetailRow name={t("issue-field.time-factor")}>
      <span {...partOf(part.nameValueWords)}>
        {factor.person === null ? t("issues.time-factor-nobody") : factorWords(factor, percent)}
      </span>
      <TimeFactorChange
        factor={factor}
        issue={blocks.issue}
        readableId={blocks.readableId}
        mayReadEstimate={blocks.mask.sees("estimated-time")}
      />
    </DetailRow>
  );
}

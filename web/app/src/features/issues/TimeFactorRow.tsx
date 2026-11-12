import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { lazy, Suspense, useState } from "react";

import type { ChangeableRowsProps } from "./ChangeableRows.tsx";
import { DetailRow } from "./DetailRow.tsx";
import { factorWords, shownPercent, useTimeFactor } from "./time-factor.ts";

/** The window loads when it first opens, so the issue page does not carry the form. */
const TimeFactorWindow = lazy(async () => ({
  default: (await import("./TimeFactorWindow.tsx")).TimeFactorWindow,
}));

/**
 * *Time factor* under *Estimated time* (PLAN-14, UX §4.4): "×1.25 · Lea's own estimate", "×1.5 · set
 * by Ada Brandt" or "×1 (as estimated)", and *Change*, which opens the window *How long will it take
 * you?*. The person on the bar always sees the row; anybody else as the field level of *Time factor*
 * allows. Whether *Change* is offered the server says (`may_change`), by the rule that also decides a
 * save. An issue assigned to nobody has no row: the factor is the person's.
 */
export function TimeFactorRow({ blocks }: Pick<ChangeableRowsProps, "blocks">) {
  const factor = useTimeFactor(blocks.readableId).data;
  const [open, setOpen] = useState(false);
  const percent = factor === undefined ? undefined : shownPercent(factor);
  if (factor === undefined || percent === undefined || factor.person === null) {
    return null;
  }
  return (
    <DetailRow name={t("issue-field.time-factor")}>
      <span {...partOf(part.nameValueWords)}>{factorWords(factor, percent)}</span>
      {factor.may_change ? (
        <Button
          look={buttonLook.quiet}
          aria-label={t("issues.time-factor-change-name")}
          onPress={() => {
            setOpen(true);
          }}
        >
          {t("issues.time-factor-change")}
        </Button>
      ) : null}
      {open ? (
        <Suspense fallback={null}>
          <TimeFactorWindow
            issue={blocks.issue}
            readableId={blocks.readableId}
            factor={factor}
            percent={percent}
            mayReadEstimate={blocks.mask.sees("estimated-time")}
            onClose={() => {
              setOpen(false);
            }}
          />
        </Suspense>
      ) : null}
    </DetailRow>
  );
}

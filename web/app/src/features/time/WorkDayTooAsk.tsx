import { t } from "@bugaboo/messages";
import { AskDialog } from "@bugaboo/ui";
import { useRef } from "react";

import { messageOf } from "../../shared/errors.ts";
import { IdsIn } from "./IdsIn.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { startWorkDay } from "./work-day-requests.ts";
import { declineWorkDayToday } from "./work-day-offer.ts";

/**
 * *Start your work day too?* (ADR-0035, decision 3): a timer was started and the work day has not.
 * Neither starts anything without the answer; *Not now* is remembered for the rest of the day, so
 * the question is not asked again with every timer.
 */
export function WorkDayTooAsk({ issue }: { readonly issue: string }) {
  const started = useRef(false);
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          if (!started.current) {
            declineWorkDayToday();
          }
          closeTimeAsk();
        }
      }}
      title={t("time.work-day.too-title")}
      text={<IdsIn sentence={t("time.work-day.too-text", { issue })} ids={[issue]} />}
      confirmLabel={t("time.work-day.too-yes")}
      cancelLabel={t("time.work-day.too-no")}
      onConfirm={() => {
        started.current = true;
        startWorkDay().catch((error: unknown) => {
          tellTime(messageOf(error), "danger");
        });
      }}
    />
  );
}

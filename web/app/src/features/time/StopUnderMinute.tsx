import { timeEntryMinutesLeast } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { AskDialog } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { IdsIn } from "./IdsIn.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { stopTimer } from "./timer-requests.ts";
import { tellSaved } from "./timer-saved.ts";

export interface StopUnderMinuteProps {
  readonly issue: string;
  /** The version of the timer that was looked at. */
  readonly version: number;
}

/**
 * *Stop timer* for a timer that ran under a minute (ADR-0035, decision 3): nothing would be saved, so
 * there is nothing to fill in. The sentence says so, *Stop timer* stops it without saving, and *Cancel*
 * leaves it running.
 */
export function StopUnderMinute({ issue, version }: StopUnderMinuteProps) {
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          closeTimeAsk();
        }
      }}
      title={<IdsIn sentence={t("time.stop.title", { issue })} ids={[issue]} />}
      text={
        <IdsIn
          sentence={t("time.stop.nothing", { issue, minutes: timeEntryMinutesLeast })}
          ids={[issue]}
        />
      }
      confirmLabel={t("time.timer.stop-timer")}
      cancelLabel={t("common.cancel")}
      onConfirm={() => {
        stopTimer(version).then(
          (saved) => {
            tellSaved(saved, issue);
          },
          (error: unknown) => {
            tellTime(messageOf(error), "danger");
          },
        );
      }}
    />
  );
}

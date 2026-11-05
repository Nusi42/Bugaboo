import { t } from "@bugaboo/messages";
import { AskDialog, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { throwAwayTimer } from "./timer-requests.ts";
import { useTimer } from "./use-timer.ts";

/** *Throw away*: the question before the timer is stopped and nothing is saved. */
export function ThrowAwayAsk() {
  const reading = useTimer().data;
  const running = reading?.timer.running ?? undefined;
  if (reading === undefined || running === undefined) {
    return null;
  }
  return (
    <AskDialog
      isOpen
      onOpenChange={closeTimeAsk}
      title={t("time.throw-away.title", {
        issue: running.readable_id ?? t("time.timer.running-hidden"),
      })}
      text={t("time.throw-away.text")}
      confirmLabel={t("time.timer.throw-away")}
      cancelLabel={t("common.cancel")}
      confirmLook={buttonLook.danger}
      onConfirm={() => {
        throwAwayTimer(reading.timer.version).catch((error: unknown) => {
          tellTime(messageOf(error), "danger");
        });
      }}
    />
  );
}

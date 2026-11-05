import { t } from "@bugaboo/messages";
import { AskDialog, Button, buttonLook } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { IdsIn } from "./IdsIn.tsx";
import { askTime, closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { tapWorkDay } from "./work-day-requests.ts";
import { useWorkDayNow } from "./use-work-days.ts";

/**
 * *Stop the timer on WEB-142 too?* (ADR-0035, decision 3): the work day ends while a timer runs.
 * *Stop the timer too* ends the day and opens *Stop timer*; *Keep it running* ends the day only;
 * *Cancel* does nothing. Neither ends anything without the answer.
 */
export function TimerTooAsk({ issue }: { readonly issue: string }) {
  const day = useWorkDayNow().now?.today;
  const end = async (): Promise<boolean> => {
    if (day === undefined) {
      return false;
    }
    try {
      await tapWorkDay("end", day.version);
      return true;
    } catch (error) {
      tellTime(messageOf(error), "danger");
      return false;
    }
  };
  return (
    <AskDialog
      isOpen
      onOpenChange={(open) => {
        if (!open) {
          closeTimeAsk();
        }
      }}
      title={<IdsIn sentence={t("time.work-day.timer-title", { issue })} ids={[issue]} />}
      text={<IdsIn sentence={t("time.work-day.timer-text", { issue })} ids={[issue]} />}
      confirmLabel={t("time.work-day.timer-yes")}
      cancelLabel={t("common.cancel")}
      onConfirm={() => {
        void end().then((ended) => {
          if (ended) {
            askTime({ kind: "stop" });
          }
        });
      }}
    >
      <Button
        look={buttonLook.plain}
        onPress={() => {
          closeTimeAsk();
          void end();
        }}
      >
        {t("time.work-day.timer-no")}
      </Button>
    </AskDialog>
  );
}

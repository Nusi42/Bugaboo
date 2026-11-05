import { t } from "@bugaboo/messages";
import { AskDialog, TextField } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { endOfOpenDay } from "./work-day-end.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { endLeftOpenDay } from "./work-day-requests.ts";
import { useWorkDayNow } from "./use-work-days.ts";

/**
 * *When did your work day on Thursday end?* (TIME-08): a day that was never ended is ended at the
 * time the person says, with the reason they give; the history of the day keeps the old times.
 */
export function OpenDayAsk() {
  const day = useWorkDayNow().now?.leftOpen;
  const [time, setTime] = useState("");
  const [reason, setReason] = useState("");
  if (day === undefined) {
    return null;
  }
  const moment = endOfOpenDay(day, time);
  const save = (): void => {
    if (moment !== null) {
      endLeftOpenDay(day, new Date(moment).toISOString(), reason.trim()).catch((error: unknown) => {
        tellTime(messageOf(error), "danger");
      });
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
      title={t("time.work-day.open-title", { day: formatCalendarDay(day.date) })}
      text={t("time.work-day.open-text")}
      confirmLabel={t("time.work-day.open-save")}
      cancelLabel={t("common.cancel")}
      confirmDisabled={moment === null || reason.trim() === ""}
      onConfirm={save}
    >
      <TextField label={t("time.work-day.open-time")} value={time} onChange={setTime} />
      <TextField label={t("time.work-day.open-reason")} value={reason} onChange={setReason} />
    </AskDialog>
  );
}

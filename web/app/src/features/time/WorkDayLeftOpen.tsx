import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook } from "@bugaboo/ui";

import { formatCalendarDay } from "../../shared/format.ts";
import { askTime } from "./time-asks.ts";
import type { WorkDay } from "./work-day-state.ts";

/** A work day of an earlier date that was never ended, with the way to say when it ended. */
export function WorkDayLeftOpen({ day }: { readonly day: WorkDay }) {
  return (
    <Alert tone={alertTone.warning}>
      <span>{t("time.work-day.left-open", { day: formatCalendarDay(day.date) })}</span>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          askTime({ kind: "open-day" });
        }}
      >
        {t("time.work-day.left-open-button")}
      </Button>
    </Alert>
  );
}

import { t } from "@bugaboo/messages";
import {
  Alert,
  alertTone,
  Button,
  buttonLook,
  Form,
  SubmitButton,
  TextField,
  partOf,
  part,
} from "@bugaboo/ui";
import { useId, useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { calendarDayOf } from "../../shared/format.ts";
import { momentOnDay, readTimeOfDay, timeBoxText } from "../../shared/wall-clock.ts";
import { EntryWindow } from "./EntryWindow.tsx";
import { closeTimeAsk } from "./time-asks.ts";
import { changeTimerStart } from "./timer-requests.ts";
import { useTimer } from "./use-timer.ts";

/** The widest span of days a day box takes: the whole calendar. */
const CALENDAR = { min: "0001-01-01", max: "9999-12-31" };

/** The new start as the API takes it, from the boxes; none while the day or the time of day is none. */
function startOf(day: string, time: string): string | undefined {
  const typed = readTimeOfDay(time);
  const moment = typed === undefined ? null : momentOnDay(day, typed);
  return moment === null ? undefined : new Date(moment).toISOString();
}

/**
 * *Change start time* (TIME-01): the day and time of day the running timer started, in the person's
 * zone. The server refuses a start in the future and one before the end of the last entry that has a
 * start, and says which.
 */
export function ChangeStartWindow() {
  const reading = useTimer().data;
  const started = reading?.timer.running?.started_at;
  const formId = useId();
  const [day, setDay] = useState(() => calendarDayOf(Date.parse(started ?? "") || Date.now()));
  const [time, setTime] = useState(() => timeBoxText(Date.parse(started ?? "") || Date.now()));
  const [problem, setProblem] = useState<string | undefined>();
  if (reading === undefined || started === undefined) {
    return null;
  }
  const save = async (): Promise<void> => {
    const moment = startOf(day, time);
    if (moment === undefined) {
      setProblem(t("time.start-time.not-a-time"));
      return;
    }
    try {
      await changeTimerStart(reading.timer.version, moment);
      closeTimeAsk();
    } catch (error) {
      setProblem(messageOf(error));
    }
  };
  return (
    <EntryWindow
      title={t("time.start-time.title")}
      isOpen
      onOpenChange={closeTimeAsk}
      footer={
        <div {...partOf(part.windowFooterActions)}>
          <Button look={buttonLook.quiet} onPress={closeTimeAsk}>
            {t("common.cancel")}
          </Button>
          <SubmitButton form={formId}>{t("time.start-time.save")}</SubmitButton>
        </div>
      }
    >
      <Form
        id={formId}
        onSubmit={() => {
          void save();
        }}
      >
        {problem === undefined ? null : <Alert tone={alertTone.danger}>{problem}</Alert>}
        <TextField label={t("time.start-time.day")} date={CALENDAR} value={day} onChange={setDay} />
        <TextField label={t("time.start-time.time")} value={time} onChange={setTime} />
      </Form>
    </EntryWindow>
  );
}

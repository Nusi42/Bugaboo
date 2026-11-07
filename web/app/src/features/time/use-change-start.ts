import { t } from "@bugaboo/messages";
import { useState } from "react";

import { exampleClock, personToday } from "../../shared/clock.ts";
import { messageOf } from "../../shared/errors.ts";
import { useRegion } from "../../shared/region.ts";
import { momentOnDay, readTimeOfDay, timeBoxText } from "../../shared/wall-clock.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { changeTimerStart } from "./timer-requests.ts";
import { useTimer } from "./use-timer.ts";

/** The new start as the API takes it, from the boxes; none while the day or the time of day is none. */
function startOf(day: string, time: string): string | undefined {
  const typed = readTimeOfDay(time);
  const moment = typed === undefined ? null : momentOnDay(day, typed);
  return moment === null ? undefined : new Date(moment).toISOString();
}

/** The boxes of *Change start time* and what saving them does. */
export interface ChangeStart {
  readonly day: string;
  readonly setDay: (day: string) => void;
  readonly time: string;
  readonly setTime: (time: string) => void;
  /** What is wrong with the boxes or what the server refused, in words. */
  readonly problem: string | undefined;
  readonly save: () => Promise<void>;
}

/**
 * The state of *Change start time*: the day and the time of day the running timer started, in the
 * person's zone. Nothing while no timer runs.
 */
export function useChangeStart(): ChangeStart | undefined {
  const reading = useTimer().data;
  const started = reading?.timer.running?.started_at;
  const region = useRegion();
  const [day, setDay] = useState(() =>
    personToday(region, Date.parse(started ?? "") || Date.now()),
  );
  const [time, setTime] = useState(() => timeBoxText(Date.parse(started ?? "") || Date.now()));
  const [problem, setProblem] = useState<string | undefined>();
  if (reading === undefined || started === undefined) {
    return undefined;
  }
  const save = async (): Promise<void> => {
    const moment = startOf(day, time);
    if (moment === undefined) {
      setProblem(
        t("time.start-time.not-a-time", {
          morning: exampleClock(8, 30),
          afternoon: exampleClock(16, 45),
        }),
      );
      return;
    }
    try {
      await changeTimerStart(reading.timer.version, moment);
      closeTimeAsk();
    } catch (error) {
      setProblem(messageOf(error));
    }
  };
  return { day, setDay, time, setTime, problem, save };
}

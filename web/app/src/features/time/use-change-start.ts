import { t } from "@bugaboo/messages";
import { useState } from "react";

import { exampleClock } from "../../shared/clock.ts";
import { calendarDayOf } from "../../shared/format.ts";
import { messageOf } from "../../shared/errors.ts";
import { type Region, useRegion } from "../../shared/region.ts";
import { momentOnDay, readTimeOfDay, timeBoxText } from "../../shared/wall-clock.ts";
import { closeTimeAsk } from "./time-asks.ts";
import { changeTimerStart } from "./timer-requests.ts";
import { useTimer } from "./use-timer.ts";

/**
 * The boxes for a start that was at `moment`: its day and its time of day on the clock of the
 * person's region. The start of a timer is a moment, so its day is the day of that moment on that
 * clock, not today (`personToday` is where a new work day starts, which is another question).
 */
export function startBoxes(
  moment: number,
  region: Region,
): { readonly day: string; readonly time: string } {
  return { day: calendarDayOf(moment, region), time: timeBoxText(moment, region) };
}

/** The new start as the API takes it, from the boxes; none while the day or the time of day is none. */
export function startOf(day: string, time: string, region: Region): string | undefined {
  const typed = readTimeOfDay(time);
  const moment = typed === undefined ? null : momentOnDay(day, typed, region);
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
  const [boxes] = useState(() => startBoxes(Date.parse(started ?? "") || Date.now(), region));
  const [day, setDay] = useState(boxes.day);
  const [time, setTime] = useState(boxes.time);
  const [problem, setProblem] = useState<string | undefined>();
  if (reading === undefined || started === undefined) {
    return undefined;
  }
  const save = async (): Promise<void> => {
    const moment = startOf(day, time, region);
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

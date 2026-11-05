import { t } from "@bugaboo/messages";
import { Button, buttonLook, Length } from "@bugaboo/ui";

import { askTime } from "./time-asks.ts";
import { useRunClock } from "./use-run-clock.ts";
import { useTimer } from "./use-timer.ts";

/** A start that is none, for the clock of a timer that does not run. */
const notRunning = { started_at: "" };

/**
 * The value of the row *Your timer* on the issue page: how long the timer has run, in the code font,
 * and *Stop*, which opens *Stop timer*. Nothing while the timer does not run.
 */
export function TimerNow() {
  const reading = useTimer().data;
  const running = reading?.timer.running ?? undefined;
  const { clock } = useRunClock(running ?? notRunning, reading?.serverOffsetMs ?? 0);
  if (running === undefined) {
    return null;
  }
  return (
    <>
      {clock === undefined ? null : <Length text={clock} />}
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          askTime({ kind: "stop" });
        }}
      >
        {t("time.timer.bar-stop")}
      </Button>
    </>
  );
}

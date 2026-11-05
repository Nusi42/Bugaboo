import { t } from "@bugaboo/messages";
import { Button, buttonLook, Length } from "@bugaboo/ui";

import { askTime } from "./time-asks.ts";
import { useRunningClock } from "./use-run-clock.ts";

/**
 * The value of the row *Your timer* on the issue page: how long the timer has run, in the code font,
 * and *Stop*, which opens *Stop timer*. Nothing while the timer does not run.
 */
export function TimerNow() {
  const { running, clock } = useRunningClock();
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

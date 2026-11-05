import { askTime } from "./time-asks.ts";
import { startTimer } from "./timer-start.ts";
import { useTimer } from "./use-timer.ts";

/** Where the timer is, seen from one issue. */
export type TimerOnIssue = "idle" | "here" | "elsewhere";

/**
 * The timer seen from one issue (TIME-01): standing still, running on this issue, or running on
 * another one; and what pressing the timer of this issue does: starting it here, opening *Stop
 * timer* when it runs here, and *Switch issue* to this issue when it runs elsewhere. The play
 * button of the issue page, the *Timer* of the bar of a phone and the key `t` all use it.
 */
export function useTimerOn(issue: string): {
  readonly state: TimerOnIssue;
  readonly press: () => void;
} {
  const running = useTimer().data?.timer.running ?? undefined;
  const state: TimerOnIssue =
    running === undefined ? "idle" : running.readable_id === issue ? "here" : "elsewhere";
  return {
    state,
    press: () => {
      if (state === "here") {
        askTime({ kind: "stop" });
      } else {
        void startTimer(issue);
      }
    },
  };
}

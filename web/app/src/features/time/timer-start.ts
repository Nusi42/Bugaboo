import { queryClient } from "../../shared/query-client.ts";
import { codeOf, messageOf } from "../../shared/errors.ts";
import { askTime } from "./time-asks.ts";
import { tellTime } from "./time-notice.ts";
import { startTimerOn, switchTimer } from "./timer-requests.ts";
import { tellSaved } from "./timer-saved.ts";
import { timerQuery } from "./use-timer.ts";

/**
 * Puts the timer on an issue (by its readable ID), whatever the timer is doing now: starts it when
 * it stands still, and when it runs on another issue switches it: the time so far is saved there
 * with its defaults and the timer goes on. When a field that must be filled in has no default on
 * that issue the *Stop timer* window opens to ask for it, and the switch goes on when it is saved.
 * Something the server refuses is told in the timer part.
 */
export async function startTimer(issue: string): Promise<void> {
  try {
    const { timer } = await queryClient.query({ ...timerQuery, staleTime: 0 });
    const running = timer.running ?? undefined;
    if (running === undefined) {
      await startTimerOn(issue);
    } else if (running.readable_id !== issue) {
      const saved = await switchTimer(timer.version, issue);
      tellSaved(saved, running.readable_id ?? issue);
    }
  } catch (error) {
    if (codeOf(error) === "field.required") {
      askTime({ kind: "stop", switchingTo: issue });
      return;
    }
    tellTime(messageOf(error), "danger");
  }
}

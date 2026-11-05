import { dataOf, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { timeKeys } from "../../shared/time-keys.ts";

/** What a stop or a switch of the timer did: the timer as it is now, and the entry it saved, if any. */
export type TimerSaved = Schema<"TimerSavedItem">;

/** Reads again what the timer, the work day and the time of issues show: something about them changed. */
export function refreshTime(): void {
  void queryClient.invalidateQueries({ queryKey: timeKeys.all() });
}

/** Reads again what an entry that was saved changes: the time of issues, their history, and the timer. */
function refreshAfterEntry(): void {
  refreshTime();
  refreshIssues();
  refreshHistory();
}

/** Starts the timer on an issue (by its readable ID). */
export async function startTimerOn(issue: string): Promise<void> {
  dataOf(await api.client.POST("/api/v1/me/timer", { body: { issue } }));
  refreshTime();
}

/** Stops the timer and saves its time as an entry with what the person filled in. */
export async function stopTimer(
  version: number,
  entry?: Schema<"EntryDetailsBody">,
): Promise<TimerSaved> {
  const saved = dataOf(
    await api.client.POST("/api/v1/me/timer/stop", {
      body: { version, ...(entry === undefined ? {} : { entry }) },
    }),
  );
  refreshAfterEntry();
  return saved;
}

/** Saves the time so far with the defaults of its issue (or with `entry`) and goes on with another issue. */
export async function switchTimer(
  version: number,
  issue: string,
  entry?: Schema<"EntryDetailsBody">,
): Promise<TimerSaved> {
  const saved = dataOf(
    await api.client.POST("/api/v1/me/timer/switch", {
      body: { version, issue, ...(entry === undefined ? {} : { entry }) },
    }),
  );
  refreshAfterEntry();
  return saved;
}

/** Sets when the running timer started. */
export async function changeTimerStart(version: number, startedAt: string): Promise<void> {
  dataOf(
    await api.client.POST("/api/v1/me/timer/start-time", {
      body: { version, started_at: startedAt },
    }),
  );
  refreshTime();
}

/** Stops the timer and saves nothing. */
export async function throwAwayTimer(version: number): Promise<void> {
  dataOf(await api.client.POST("/api/v1/me/timer/throw-away", { body: { version } }));
  refreshTime();
}

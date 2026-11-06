import { dataOf } from "@bugaboo/api-client";

import { codeOf, type Context } from "./context.ts";
import { pick } from "./random.ts";
import { type Work } from "./book.ts";

// Timers and work days at the same moments (architecture §11, "Timers and working hours start and
// stop at the same time"): the people start and stop their timers, and tap their days, while the
// others work. What the server kept is written in the book, and `bb-load-check` judges it.

/** Refusals that are the rules at work, not a defect: another change a second too soon, or a day or timer that is not as the person thought. */
const RULES = new Set([
  "timer.too-fast",
  "timer.running",
  "limit.reached",
  "working-hours.day-running",
  "working-hours.day-not-running",
  "working-hours.break-running",
  "working-hours.no-break-running",
  "working-hours.day-left-open",
  "edit.version-changed",
]);

/** How far back the start of a timer is moved before it is stopped, in minutes. */
const RAN_MINUTES = 5;
const MS_PER_MINUTE = 60_000;

/** The version of the timer after its start was moved back, or the one it had when the rules said no. */
async function movedBack(context: Context, version: number): Promise<number> {
  try {
    await context.client.POST("/api/v1/me/timer/start-time", {
      body: {
        version,
        started_at: new Date(Date.now() - RAN_MINUTES * MS_PER_MINUTE).toISOString(),
      },
    });
    return dataOf(await context.client.GET("/api/v1/me/timer")).version;
  } catch (error) {
    if (!(codeOf(error) ?? "").startsWith("timer.")) {
      throw error;
    }
    return version;
  }
}

/**
 * Starts the person's timer on the issue, or stops the one that runs: a stop that saved time is
 * written down, and the database must hold it once.
 */
export async function timeNow(context: Context, work: Work): Promise<void> {
  const timer = dataOf(await context.client.GET("/api/v1/me/timer"));
  if (timer.running === undefined || timer.running === null) {
    try {
      await context.client.POST("/api/v1/me/timer", { body: { issue: work.key } });
    } catch (error) {
      if (!RULES.has(codeOf(error) ?? "")) {
        throw error;
      }
    }
    return;
  }
  try {
    // A timer under a minute saves nothing: the start is moved back, once the rules allow it (not
    // before the end of the last entry a timer saved).
    const version = await movedBack(context, timer.version);
    const stopped = dataOf(
      await context.client.POST("/api/v1/me/timer/stop", { body: { version } }),
    );
    const key = stopped.entry?.readable_id;
    if (typeof key === "string") {
      context.book.timed(key);
    }
  } catch (error) {
    if (!RULES.has(codeOf(error) ?? "")) {
      throw error;
    }
  }
}

/** What the person's day was last, so the next tap is a sensible one. */
const states = new Map<number, { readonly date: string; readonly version: number }>();

/** One tap on the day that runs: starts a break, ends it, or ends the day. */
async function tap(context: Context, version: number) {
  const body = { version };
  const which = pick(context.random, 3);
  if (which === 0) {
    return dataOf(await context.client.POST("/api/v1/me/work-days/break/start", { body }));
  }
  return which === 1
    ? dataOf(await context.client.POST("/api/v1/me/work-days/break/end", { body }))
    : dataOf(await context.client.POST("/api/v1/me/work-days/end", { body }));
}

/**
 * Taps the person's day: starts it, then breaks and ends it, over and over. Every change the server
 * kept is written in the book as a version of that day.
 */
export async function dayNow(context: Context): Promise<void> {
  const known = states.get(context.number);
  try {
    const changed =
      known === undefined || pick(context.random, 4) === 0
        ? dataOf(await context.client.POST("/api/v1/me/work-days/start", { body: {} }))
        : await tap(context, known.version);
    states.set(context.number, { date: changed.date, version: changed.version });
    context.book.dayed({ person: context.holder, date: changed.date });
  } catch (error) {
    if (!RULES.has(codeOf(error) ?? "")) {
      throw error;
    }
    states.delete(context.number);
  }
}

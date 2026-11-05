import type { Schema } from "@bugaboo/api-client";

/** One work day as the server shows it. */
export type WorkDay = Schema<"WorkDayBody">;

/** What the work day of today is doing. */
export type TodayKind = "none" | "working" | "break" | "ended";

/** What the person's days say right now. */
export interface WorkDayNow {
  /** What today is doing; `none` while no work day of today has started. */
  readonly kind: TodayKind;
  /** The work day of today, if one has started. */
  readonly today: WorkDay | undefined;
  /** An older day that was never ended, the first one: the question *When did your work day on … end?*. */
  readonly leftOpen: WorkDay | undefined;
}

/** Whether a stretch of a day is still running. */
function runs(spans: readonly Schema<"SpanBody">[]): boolean {
  return spans.some((span) => span.end === null);
}

/** What a work day of today is doing: on a break, working, or over. */
function kindOf(day: WorkDay): Exclude<TodayKind, "none"> {
  if (!day.runs) {
    return "ended";
  }
  return runs(day.breaks) ? "break" : "working";
}

/**
 * What the days of the last week say about now: what today is doing, and the first older day that
 * is still running (the server never ends a day by itself). The state is only read from what the
 * server sent; nothing is worked out again.
 */
export function workDayNow(days: readonly WorkDay[], today: string): WorkDayNow {
  const ofToday = days.find((day) => day.date === today);
  const leftOpen = days.find((day) => day.runs && day.date < today);
  return {
    kind: ofToday === undefined ? "none" : kindOf(ofToday),
    today: ofToday,
    leftOpen,
  };
}

/** The first start of a day, as the server wrote it; none for a day without a period. */
export function startOf(day: WorkDay): string | undefined {
  return day.periods[0]?.start;
}

/** The last end of a day; none while it runs or for a day without a period. */
export function endOf(day: WorkDay): string | undefined {
  return day.periods.at(-1)?.end ?? undefined;
}

/** The start of the break that runs; none when no break runs. */
export function breakStartOf(day: WorkDay): string | undefined {
  return day.breaks.find((span) => span.end === null)?.start;
}

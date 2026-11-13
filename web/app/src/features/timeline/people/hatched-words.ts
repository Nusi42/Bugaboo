import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { daysBetween } from "../../../shared/clock.ts";

/** A hatched day of a bar, as the server answers it for the reader. */
export type HatchedDay = Schema<"HatchedDayBody">;

/**
 * What a hatched day is called: *Away* (only ever that, never why), a holiday by its name, not a
 * work day, or, for a person the reader does not see, only *Day without work*: the server sends that
 * one neutral kind for all three, so nobody learns that a day was an absence (ADR-0030, amendment
 * M8).
 */
export function hatchedText(day: HatchedDay): string | undefined {
  if (day.kind === "no-work") {
    return t("timeline.legend-no-work");
  }
  if (day.kind === "holiday" && day.holiday !== null) {
    return t("timeline.holiday", { name: t(`holiday.${day.holiday}`) });
  }
  if (day.kind === "day-off") {
    return t("timeline.day-off");
  }
  if (day.morning && !day.afternoon) {
    return t("timeline.away-morning");
  }
  if (day.afternoon && !day.morning) {
    return t("timeline.away-afternoon");
  }
  return day.morning ? t("timeline.away") : undefined;
}

/** Days of a bar next to each other that look the same and are called the same. */
export interface HatchedStretch {
  readonly kind: HatchedDay["kind"];
  /** Days after the first day of the bar. */
  readonly start: number;
  readonly length: number;
  /** What the days are called; none where the server sends no word. */
  readonly words: string | undefined;
}

/** The hatched days of a bar as stretches, from its first day: the days come from the server, in order. */
export function hatchedStretches(
  days: readonly HatchedDay[],
  first: string,
): readonly HatchedStretch[] {
  const found: HatchedStretch[] = [];
  for (const day of days) {
    const start = daysBetween(first, day.date);
    const words = hatchedText(day);
    const last = found.at(-1);
    if (
      last !== undefined &&
      last.kind === day.kind &&
      last.words === words &&
      last.start + last.length === start
    ) {
      found[found.length - 1] = { ...last, length: last.length + 1 };
    } else {
      found.push({ kind: day.kind, start, length: 1, words });
    }
  }
  return found;
}

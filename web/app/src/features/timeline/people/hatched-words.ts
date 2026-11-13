import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { daysBetween } from "../../../shared/clock.ts";

/** A hatched day of a bar, as the server answers it for the reader. */
export type HatchedDay = Schema<"HatchedDayBody">;

/**
 * What a hatched day is called: *Away* (only ever that, never why), a holiday by its name, or not a
 * work day. A bar whose person the reader may not see has no word on its days (ADR-0030).
 */
export function hatchedText(day: HatchedDay, named: boolean): string | undefined {
  if (!named) {
    return undefined;
  }
  if (day.kind === "holiday") {
    return day.holiday === null
      ? t("timeline.legend-holiday")
      : t("timeline.holiday", { name: t(`holiday.${day.holiday}`) });
  }
  if (day.kind === "day-off") {
    return t("timeline.day-off");
  }
  if (day.morning && !day.afternoon) {
    return t("timeline.away-morning");
  }
  return day.afternoon && !day.morning ? t("timeline.away-afternoon") : t("timeline.away");
}

/** Days of a bar next to each other that look the same and are called the same. */
export interface HatchedStretch {
  readonly kind: HatchedDay["kind"];
  /** Days after the first day of the bar. */
  readonly start: number;
  readonly length: number;
  /** What the days are called; none for a bar whose person the reader may not see. */
  readonly words: string | undefined;
}

/** The hatched days of a bar as stretches, from its first day: the days come from the server, in order. */
export function hatchedStretches(
  days: readonly HatchedDay[],
  first: string,
  named: boolean,
): readonly HatchedStretch[] {
  const found: HatchedStretch[] = [];
  for (const day of days) {
    const start = daysBetween(first, day.date);
    const words = hatchedText(day, named);
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

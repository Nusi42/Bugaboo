import { addDays } from "../../shared/format.ts";
import type { KeptDates } from "./date-fields.ts";
import type { BarDays } from "./lines.ts";

/** What a drag or a key does to a bar: moves it, or moves one of its ends. */
export type Move = "both" | "start" | "end";

/**
 * The dates an issue keeps after its bar moved by `days`: a move shifts every date it keeps, an end
 * that is moved gets the day the bar shows there moved (also a day the plan worked out, which is
 * then kept). An end never passes the other one: the server would refuse it.
 */
export function moved(kept: KeptDates, bar: BarDays, move: Move, days: number): KeptDates {
  if (move === "both") {
    return {
      start: kept.start === null ? null : addDays(kept.start, days),
      end: kept.end === null ? null : addDays(kept.end, days),
    };
  }
  if (move === "start") {
    const start = addDays(bar.first, days);
    return { ...kept, start: start > bar.last ? bar.last : start };
  }
  const end = addDays(bar.last, days);
  return { ...kept, end: end < bar.first ? bar.first : end };
}

/** The days a bar shows while its dates change, until the server answers with its own. */
export function shownWhileMoving(bar: BarDays, move: Move, days: number): BarDays {
  const first = move === "end" ? bar.first : addDays(bar.first, days);
  const last = move === "start" ? bar.last : addDays(bar.last, days);
  if (first <= last) {
    return { first, last };
  }
  // An end dragged past the other one stops there, as the saved dates do.
  return move === "start" ? { first: last, last } : { first, last: first };
}

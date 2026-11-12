import type { Move } from "./dates.ts";

/** What a key of a selected bar asks: the keys every drag of a bar has (TEST-A11Y, UX §4.7). */
export interface KeyMove {
  readonly move: Move;
  readonly days: number;
}

/** The keys of a bar, what they hold. */
interface BarKey {
  readonly key: string;
  readonly altKey: boolean;
  readonly shiftKey: boolean;
}

const daysPerWeek = 7;

/** The way an arrow key points along the days: later, earlier, or not along them. */
const along: Readonly<Record<string, 1 | -1>> = { ArrowRight: 1, ArrowLeft: -1 };

/**
 * The move a key asks of a bar: an arrow moves it a day, with *Shift* a week; with *Alt* it moves
 * the due date (the end) a day, with *Alt* and *Shift* the start date. Any other key asks nothing.
 */
export function keyMoveOf(event: BarKey): KeyMove | undefined {
  const step = along[event.key];
  if (step === undefined) {
    return undefined;
  }
  if (event.altKey) {
    return { move: event.shiftKey ? "start" : "end", days: step };
  }
  return { move: "both", days: event.shiftKey ? step * daysPerWeek : step };
}

import { useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/**
 * What the timer and the work day are asking the person, in a window or a question over the page.
 * One at a time: a window of time that opens replaces the one before.
 */
export type TimeAsk =
  /** *Stop timer*; with `switchingTo`, the time so far is saved because the timer goes on another issue. */
  | { readonly kind: "stop"; readonly switchingTo?: string }
  /** *Switch issue*: which issue to go on with. */
  | { readonly kind: "switch" }
  /** *Change start time*. */
  | { readonly kind: "start-time" }
  /** *Throw away*, after a question. */
  | { readonly kind: "throw-away" };

const current = createStore<TimeAsk | null>(null);

/** Opens a window or a question of time. */
export function askTime(ask: TimeAsk): void {
  current.replace(ask);
}

/** Closes it, whichever it was. */
export function closeTimeAsk(): void {
  current.replace(null);
}

/** What is asked now, if anything. */
export function useTimeAsk(): TimeAsk | null {
  return useSyncExternalStore(current.subscribe, current.snapshot);
}

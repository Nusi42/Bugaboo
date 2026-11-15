import { createContext, useContext, useId, useLayoutEffect } from "react";

import { doNothing } from "./do-nothing.ts";

/** Tells the hold around a part whether the part still waits for its first read. */
export type Report = (id: string, waiting: boolean) => void;

/** The hold around a part: how a part tells it that it waits, and whether it holds now. */
export interface Hold {
  readonly report: Report;
  readonly held: boolean;
}

export const HoldContext = createContext<Hold | null>(null);

/** What a screen reader hears while a hold waits (the app gives its words for *Loading…* at its root). */
export const HoldWords = createContext("");

/**
 * Whether the hold around a part holds now: a part that draws what it holds somewhere else (into the head of its pane)
 * hides it there too, the same way. Outside a hold it never holds.
 */
export function useHeld(): boolean {
  return useContext(HoldContext)?.held === true;
}

/**
 * Says that the part waits for its first read while `waiting`: the hold around it keeps everything it holds hidden
 * until every part in it has its first answer, then shows them all at once, each in its place (UX §6.4). Outside a hold
 * it does nothing.
 */
export function useHoldWhileReading(waiting: boolean): void {
  const report = useContext(HoldContext)?.report ?? null;
  const id = useId();
  useLayoutEffect(() => {
    if (report === null) {
      return doNothing;
    }
    report(id, waiting);
    return () => {
      report(id, false);
    };
  }, [report, id, waiting]);
}

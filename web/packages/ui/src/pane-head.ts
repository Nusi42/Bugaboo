import { createContext, useContext } from "react";

/** The head of the pane a part stands in: where deeper parts put what belongs to it, and where it ends whole. */
export interface PaneHead {
  readonly slot: HTMLElement | null;
  readonly above: number;
}

/** Outside a pane: no head to put anything into, and no room to keep above a box. */
export const PaneHeadContext = createContext<PaneHead>({ slot: null, above: 0 });

/**
 * How far down the pane the whole head ends, as the browser draws it: the room a box of the pane keeps above what it
 * holds when it lays itself out in code (a list drawn only where it is on screen). Outside a pane it is 0.
 */
export function usePaneHeadAbove(): number {
  return useContext(PaneHeadContext).above;
}

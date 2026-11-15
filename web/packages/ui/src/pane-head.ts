import { createContext, useContext, useSyncExternalStore, type ReactNode } from "react";
import { createPortal } from "react-dom";

/**
 * The places in the head of a pane that parts deeper in the pane put their lines into, in the order they stand there:
 * the heading and the introduction of the page (a topic of the settings) before what the head holds itself, then the
 * search, then how the results are shown (the options and the counts of a list, the tools of the timeline).
 */
export const paneHeadSlot = { title: "title", search: "search", view: "view" } as const;
export type PaneHeadSlot = (typeof paneHeadSlot)[keyof typeof paneHeadSlot];

/** Whether the head of a pane folds over its boxes as they scroll (a computer), or stands in the page above them. */
export const paneLook = { folds: "folds", still: "still" } as const;
export type PaneLook = (typeof paneLook)[keyof typeof paneLook];

/** The head of the pane a part stands in: where deeper parts put what belongs to it, and where it ends whole. */
export interface PaneHead {
  /** The element of each slot once it is drawn; none outside a pane. */
  readonly slots: Readonly<Record<PaneHeadSlot, HTMLElement | null>> | undefined;
  readonly above: number;
}

/** Outside a pane: no head to put anything into, and no room to keep above a box. */
export const OUTSIDE_PANE: PaneHead = { slots: undefined, above: 0 };

export const PaneHeadContext = createContext<PaneHead>(OUTSIDE_PANE);

/** Whether a part stands in a pane, so what belongs above its boxes goes into the head of the pane. */
export function useInPane(): boolean {
  return useContext(PaneHeadContext).slots !== undefined;
}

/**
 * How far down the pane the whole head ends, as the browser draws it: the room a box of the pane keeps above what it
 * holds when it lays itself out in code (a list drawn only where it is on screen). Outside a pane, on paper and where
 * the head does not fold it is 0.
 */
export function usePaneHeadAbove(): number {
  return useContext(PaneHeadContext).above;
}

/** The query of the medium of paper. */
const paper = "print";

function subscribePaper(listener: () => void): () => void {
  const watched = matchMedia(paper);
  watched.addEventListener("change", listener);
  return () => {
    watched.removeEventListener("change", listener);
  };
}

function readPaper(): boolean {
  return matchMedia(paper).matches;
}

function serverPaper(): boolean {
  return false;
}

/** Whether the page is being drawn for paper: what code lays out follows the rules of the theme for print. */
export function useOnPaper(): boolean {
  return useSyncExternalStore(subscribePaper, readPaper, serverPaper);
}

/**
 * Puts what a part deeper in the pane draws above its box (the search bar, the options of a list) into its slot of the
 * head of the pane, so it folds with the head (UX §6.5). Outside a pane it stands where it is; in a pane it waits for
 * its slot, so it is drawn once, in its place.
 */
export function InPaneHead({
  slot,
  children,
}: {
  readonly slot: PaneHeadSlot;
  readonly children: ReactNode;
}): ReactNode {
  const { slots } = useContext(PaneHeadContext);
  if (slots === undefined) {
    return children;
  }
  const into = slots[slot];
  return into === null ? null : createPortal(children, into);
}

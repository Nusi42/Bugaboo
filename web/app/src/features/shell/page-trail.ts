import { useEffect } from "react";

import { createRegistry } from "./registry.ts";

/** One step of *where you are*: what it is called and where it goes. */
export interface TrailStep {
  readonly label: string;
  /** An address of the app. */
  readonly href: string;
}

/**
 * *Where you are* of a page that is not one of the fixed screens (a project): the steps from the
 * top down, the last one being the page itself. A page that has one says so while it is on screen;
 * the top bar then shows it instead of the trail of the fixed screens.
 */
export interface PageTrail {
  readonly steps: readonly TrailStep[];
}

const trails = createRegistry<PageTrail>();

/** The trails the open pages set; the latest one counts. */
export const pageTrails = trails;

/**
 * Puts the trail of the open page into the top bar for as long as the page is on screen. The
 * steps are kept as they are (`useMemo`), so a page that makes a new array every time would put
 * its trail in again and again.
 */
export function usePageTrail(steps: readonly TrailStep[] | undefined): void {
  useEffect(() => {
    const remove = steps === undefined ? undefined : trails.add({ steps });
    return () => {
      remove?.();
    };
  }, [steps]);
}

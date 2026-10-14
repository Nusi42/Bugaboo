import { useSyncExternalStore } from "react";

import { breakpoints } from "./tokens.ts";

/** The three sizes of screen the layout is made for (UX §3). */
export const layoutModes = ["phone", "tablet", "computer"] as const;
export type LayoutMode = (typeof layoutModes)[number];

/** The widest the window can be, as the layout sees it: a phone is narrower than a tablet. */
const queries = {
  computer: `(min-width: ${String(breakpoints.computer)}px)`,
  tablet: `(min-width: ${String(breakpoints.tablet)}px)`,
} as const;

/** The layout for a window of this width in pixels. */
export function layoutForWidth(width: number): LayoutMode {
  if (width >= breakpoints.computer) {
    return "computer";
  }
  return width >= breakpoints.tablet ? "tablet" : "phone";
}

function readLayout(): LayoutMode {
  if (matchMedia(queries.computer).matches) {
    return "computer";
  }
  return matchMedia(queries.tablet).matches ? "tablet" : "phone";
}

function subscribe(listener: () => void): () => void {
  const watched = [matchMedia(queries.computer), matchMedia(queries.tablet)];
  for (const query of watched) {
    query.addEventListener("change", listener);
  }
  return () => {
    for (const query of watched) {
      query.removeEventListener("change", listener);
    }
  };
}

const upright = "(orientation: portrait)";

function readUpright(): boolean {
  return matchMedia(upright).matches;
}

function subscribeUpright(listener: () => void): () => void {
  const watched = matchMedia(upright);
  watched.addEventListener("change", listener);
  return () => {
    watched.removeEventListener("change", listener);
  };
}

/** Whether the window is held upright (taller than wide); it follows the window when it is turned. */
export function useUpright(): boolean {
  return useSyncExternalStore(subscribeUpright, readUpright);
}

/** The layout of the window right now; it follows the window when it is resized or turned. */
export function useLayout(): LayoutMode {
  return useSyncExternalStore(subscribe, readLayout);
}

import { useEffect, useSyncExternalStore } from "react";

/** Remembers on this device whether the person narrowed the sidebar to icons. */
const storageKey = "bugaboo.sidebar-narrow";

interface SidebarState {
  /** The person narrowed it. */
  readonly narrowed: boolean;
  /** Screens that need room (the focus view) narrow it while they are open. */
  readonly squeezed: number;
}

function readNarrowed(): boolean {
  try {
    return localStorage.getItem(storageKey) === "yes";
  } catch {
    return false;
  }
}

let state: SidebarState = { narrowed: readNarrowed(), squeezed: 0 };
const listeners = new Set<() => void>();

function change(next: SidebarState): void {
  state = next;
  for (const listener of listeners) {
    listener();
  }
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function snapshot(): SidebarState {
  return state;
}

/** Whether the sidebar shows icons only, by the person's choice or because a screen needs room. */
export function useSidebarNarrow(): {
  readonly narrow: boolean;
  /** A screen has narrowed it (the focus view): the person cannot widen it until that screen is closed. */
  readonly locked: boolean;
} {
  const current = useSyncExternalStore(subscribe, snapshot);
  return { narrow: current.narrowed || current.squeezed > 0, locked: current.squeezed > 0 };
}

/** Narrows or widens the sidebar, and remembers it. */
export function toggleSidebar(): void {
  const narrowed = !state.narrowed;
  try {
    localStorage.setItem(storageKey, narrowed ? "yes" : "no");
  } catch {
    // The choice then only lasts until the page is closed.
  }
  change({ ...state, narrowed });
}

/** Narrows the sidebar while `active`: the focus view gives the open item most of the screen. */
export function useNarrowSidebarWhile(active: boolean): void {
  useEffect(() => {
    if (active) {
      change({ ...state, squeezed: state.squeezed + 1 });
    }
    return () => {
      if (active) {
        change({ ...state, squeezed: state.squeezed - 1 });
      }
    };
  }, [active]);
}

import { readStored, storageKey, writeStored } from "@bugaboo/ui";
import { useEffect, useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/** Remembers on this device whether the person narrowed the sidebar to icons. */
const NARROW = "yes";
const WIDE = "no";

interface SidebarState {
  /** The person narrowed it. */
  readonly narrowed: boolean;
  /** Screens that need room (the focus view) narrow it while they are open. */
  readonly squeezed: number;
  /** Focus views that are open, on any screen: a phone has no sidebar to narrow, but has the view. */
  readonly focused: number;
}

const sidebar = createStore<SidebarState>({
  narrowed: readStored(storageKey.sidebarNarrow) === NARROW,
  squeezed: 0,
  focused: 0,
});

/** Whether the sidebar shows icons only, by the person's choice or because a screen needs room. */
export function useSidebarNarrow(): {
  readonly narrow: boolean;
  /** A screen has narrowed it (the focus view): the person cannot widen it until that screen is closed. */
  readonly locked: boolean;
  /** A focus view is open: it has bars of its own, so the page has no top bar on a phone. */
  readonly focus: boolean;
} {
  const current = useSyncExternalStore(sidebar.subscribe, sidebar.snapshot);
  return {
    narrow: current.narrowed || current.squeezed > 0,
    locked: current.squeezed > 0,
    focus: current.focused > 0,
  };
}

/** Narrows or widens the sidebar, and remembers it. */
export function toggleSidebar(): void {
  const narrowed = !sidebar.snapshot().narrowed;
  writeStored(storageKey.sidebarNarrow, narrowed ? NARROW : WIDE);
  sidebar.replace({ ...sidebar.snapshot(), narrowed });
}

/** Narrows the sidebar while `active`: the focus view gives the open item most of the screen. */
export function useNarrowSidebarWhile(active: boolean): void {
  useEffect(() => {
    if (active) {
      sidebar.replace({ ...sidebar.snapshot(), squeezed: sidebar.snapshot().squeezed + 1 });
    }
    return () => {
      if (active) {
        sidebar.replace({ ...sidebar.snapshot(), squeezed: sidebar.snapshot().squeezed - 1 });
      }
    };
  }, [active]);
}

/** Notes that a focus view is open while `active`, whatever the screen. */
export function useFocusViewOpen(active: boolean): void {
  useEffect(() => {
    if (active) {
      sidebar.replace({ ...sidebar.snapshot(), focused: sidebar.snapshot().focused + 1 });
    }
    return () => {
      if (active) {
        sidebar.replace({ ...sidebar.snapshot(), focused: sidebar.snapshot().focused - 1 });
      }
    };
  }, [active]);
}

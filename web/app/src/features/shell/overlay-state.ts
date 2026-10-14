import { useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/** The layers that open over the page; one at a time. */
export const overlays = {
  search: "search",
  shortcuts: "shortcuts",
  menu: "menu",
  more: "more",
  newIssue: "new-issue",
} as const;

/** The name of a layer. */
export type OverlayName = (typeof overlays)[keyof typeof overlays];

const open = createStore<OverlayName | null>(null);

/** Opens a layer; one that was open closes. */
export function openOverlay(name: OverlayName): void {
  open.replace(name);
}

/** Closes the layer that is open. */
export function closeOverlay(): void {
  open.replace(null);
}

/** Whether this layer is open. */
export function useOverlayOpen(name: OverlayName): boolean {
  return useSyncExternalStore(open.subscribe, () => open.snapshot() === name);
}

/** The functions to hand to an `Overlay`. */
export function useOverlay(name: OverlayName): {
  readonly isOpen: boolean;
  readonly onOpenChange: (isOpen: boolean) => void;
} {
  const isOpen = useOverlayOpen(name);
  return {
    isOpen,
    onOpenChange: (next) => {
      if (next) {
        openOverlay(name);
      } else {
        closeOverlay();
      }
    },
  };
}

import { useSyncExternalStore } from "react";

/** The layers that open over the page; one at a time. */
export type OverlayName = "search" | "shortcuts" | "menu" | "more";

let open: OverlayName | null = null;
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function setOpen(next: OverlayName | null): void {
  open = next;
  for (const listener of listeners) {
    listener();
  }
}

/** Opens a layer; one that was open closes. */
export function openOverlay(name: OverlayName): void {
  setOpen(name);
}

/** Closes the layer that is open. */
export function closeOverlay(): void {
  setOpen(null);
}

/** Whether this layer is open. */
export function useOverlayOpen(name: OverlayName): boolean {
  return useSyncExternalStore(subscribe, () => open === name);
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

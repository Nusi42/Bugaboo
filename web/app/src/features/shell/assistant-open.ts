import { useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/**
 * Whether the assistant panel is open. It is its own state and not one of the layers over the page:
 * on a computer the panel stands beside the page without dimming it (UX §4.12), so opening quick
 * search or a dialog never closes it, and a conversation is never dropped by something else opening.
 */
const open = createStore(false);

/** Opens the assistant panel. */
export function openAssistantPanel(): void {
  open.replace(true);
}

/** Closes the assistant panel; its conversation ends with it. */
export function closeAssistantPanel(): void {
  open.replace(false);
}

/** Whether the assistant panel is open. */
export function useAssistantPanelOpen(): boolean {
  return useSyncExternalStore(open.subscribe, open.snapshot);
}

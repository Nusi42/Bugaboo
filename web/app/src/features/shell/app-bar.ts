import { useEffect, useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

/** How many screens hide the bar of the app now: two at once (a screen that opens before the last one closes) keep it hidden. */
const hiding = createStore(0);

/**
 * Hides the bar of the app on a phone for as long as the calling screen is on show (M5-F172): an
 * open issue brings a bar of its own (*Status · Comment · Timer · More*), and two bars under one page
 * would show *More* twice. The page leads out with *Back* in its top bar.
 */
export function useHideAppBar(): void {
  useEffect(() => {
    hiding.replace(hiding.snapshot() + 1);
    return () => {
      hiding.replace(hiding.snapshot() - 1);
    };
  }, []);
}

/** Whether a screen has hidden the bar of the app. */
export function useAppBarHidden(): boolean {
  return useSyncExternalStore(hiding.subscribe, () => hiding.snapshot() > 0);
}

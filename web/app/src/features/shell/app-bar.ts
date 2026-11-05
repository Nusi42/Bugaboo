import { useEffect, useSyncExternalStore } from "react";

import { createStore } from "../../shared/store.ts";

const hidden = createStore(false);

/**
 * Hides the bar of the app on a phone for as long as the calling screen is on show (M5-F172): an
 * open issue brings a bar of its own (*Status · Comment · Timer · More*), and two bars under one page
 * would show *More* twice. The page leads out with *Back* in its top bar.
 */
export function useHideAppBar(): void {
  useEffect(() => {
    hidden.replace(true);
    return () => {
      hidden.replace(false);
    };
  }, []);
}

/** Whether a screen has hidden the bar of the app. */
export function useAppBarHidden(): boolean {
  return useSyncExternalStore(hidden.subscribe, hidden.snapshot);
}

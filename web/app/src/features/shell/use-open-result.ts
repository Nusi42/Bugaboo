import { useRouter } from "@tanstack/react-router";

import { closeOverlay } from "./overlay-state.ts";
import { resultByKey, type ResultGroup } from "./quick-search-collect.ts";

/**
 * Opens a result by its name. Opening closes the dialog, also when the result is the page that is
 * open already. An action closes it first, so an action that opens a dialog of its own is not
 * closed with it.
 */
export function useOpenResult(groups: readonly ResultGroup[]): (key: string | number) => void {
  const router = useRouter();
  return (key) => {
    const hit = resultByKey(groups, key);
    if (hit?.run !== undefined) {
      closeOverlay();
      hit.run();
      return;
    }
    if (hit !== undefined) {
      router.history.push(hit.path);
    }
    closeOverlay();
  };
}

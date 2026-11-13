import { doNothing } from "./do-nothing.ts";

/** How long the page is left alone before code that a dialog needs later is fetched. */
export const idlePreloadMs = 2000;

/**
 * Fetches something in the background after a pause, once: the code of a dialog that is loaded
 * when it is first opened (the inside of quick search, the new-issue window), so it is there when
 * the person asks for it. A fetch that fails is left alone; the dialog asks again when it opens.
 * Returns the way to stop before the pause is over.
 */
export function startPreload(load: () => Promise<unknown>, afterMs: number): () => void {
  const timer = setTimeout(() => {
    void load().catch(doNothing);
  }, afterMs);
  return () => {
    clearTimeout(timer);
  };
}

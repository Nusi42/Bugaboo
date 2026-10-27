import { doNothing } from "../../shared/do-nothing.ts";

/**
 * Fetches something in the background after a pause, once: the inside of the dialog of quick
 * search, so it is there at the first key press. A fetch that fails is left alone; the dialog
 * asks again when it opens. Returns the way to stop before the pause is over.
 */
export function startPreload(load: () => Promise<unknown>, afterMs: number): () => void {
  const timer = setTimeout(() => {
    void load().catch(doNothing);
  }, afterMs);
  return () => {
    clearTimeout(timer);
  };
}

import { useEffect } from "react";

/**
 * Runs `measure` now and whenever one of the elements that `watched` names changes size (the page is
 * resized, the content grows). Both functions must stay the same between renders (`useCallback`) or
 * the watch starts over each time.
 */
export function useOnResize(measure: () => void, watched: () => readonly (Element | null)[]): void {
  useEffect(() => {
    measure();
    const watch = new ResizeObserver(measure);
    for (const target of watched()) {
      if (target !== null) {
        watch.observe(target);
      }
    }
    return () => {
      watch.disconnect();
    };
  }, [measure, watched]);
}

import { useCallback, useEffect, useRef } from "react";

import { historyStep, type HistoryStep } from "../../shared/search-address.ts";

/** Takes every filter out of the search: the same function on every draw, whatever `onText` is now. */
export function useClearFilters(
  onText: (canonical: string, step: HistoryStep) => void,
): () => void {
  // The latest `onText`, so the function that clears the filters does not change.
  const latest = useRef(onText);
  useEffect(() => {
    latest.current = onText;
  });
  return useCallback(() => {
    latest.current("", historyStep.push);
  }, []);
}

import { useState } from "react";

import { useMistakes } from "../../shared/mistakes.ts";
import type { TimeEntry } from "./edits.ts";
import { useChangeEntry } from "./use-change-entry.ts";

/**
 * The entry picked in the details of a row, its change, and the mistakes of its form: the window
 * owns the footer with *Save*, so it holds them, and forgets them when another entry is picked or
 * none is (UX §6, a form sent as a whole holds *Save* while a box holds no value yet).
 */
export function usePickedEntry() {
  const [picked, setPickedEntry] = useState<TimeEntry | undefined>();
  const mistakes = useMistakes();
  const setPicked = (entry: TimeEntry | undefined): void => {
    mistakes.clear();
    setPickedEntry(entry);
  };
  const changing = useChangeEntry(() => {
    setPicked(undefined);
  });
  return { picked, setPicked, mistakes, ...changing };
}

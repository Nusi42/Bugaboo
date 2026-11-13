import { useEffect, useRef, useState } from "react";

import { useLetGoWhenHidden } from "../../shared/editing/index.ts";
import { browserSchedule } from "@bugaboo/ui";
import { keyRuns, runBook } from "./key-runs.ts";
import type { TimelineRow } from "./timeline-data.ts";
import type { Saving } from "./use-saving.ts";

/** The runs of keys of this screen (see `keyRuns`), kept across its draws. */
export function useKeyRuns(saving: Readonly<Saving>) {
  const [runs] = useState(runBook<TimelineRow>);
  return keyRuns(runs, saving, browserSchedule);
}

/** A page that comes back from the cache of the browser has nothing to take again. */
function nothingToTakeAgain(): void {
  // The runs were saved when the page was hidden; the screen shows what the server answers.
}

/**
 * Saves the runs of keys that wait when the page is hidden (`pagehide`: closed, reloaded, left)
 * and when the timeline goes away inside the app, the way an editor lets its lock go. Every save
 * of dates is a `keepalive` request (`dates.ts`), so the browser still sends it after the page is
 * gone.
 *
 * What can be lost: the saves of one issue go one after the other. When a save of the same issue is
 * still on its way as the page goes, the run waits behind it, and the page is gone before the run
 * is sent: the server keeps the change before the run. Inside the app nothing is lost, as the
 * queue goes on after the timeline is gone.
 */
export function useSaveOnLeave(endAll: () => void): void {
  const latest = useRef(endAll);
  useEffect(() => {
    latest.current = endAll;
  });
  const [letGo] = useState(() => () => {
    latest.current();
  });
  useLetGoWhenHidden(letGo, nothingToTakeAgain);
}

import { versionsOf, watchSeconds } from "@bugaboo/api-client";
import { useEffect, useRef } from "react";

import { api } from "../api.ts";
import { startWatching, type Watched, type WatchEnv } from "./version-watch.ts";

/** The browser behind the check of versions. */
const browser: WatchEnv = {
  onWindow: (name, run) => {
    window.addEventListener(name, run);
    return () => {
      window.removeEventListener(name, run);
    };
  },
  onDocument: (name, run) => {
    document.addEventListener(name, run);
    return () => {
      document.removeEventListener(name, run);
    };
  },
  hidden: () => document.visibilityState === "hidden",
  repeat: (seconds, run) => {
    const timer = setInterval(run, seconds * 1000);
    return () => {
      clearInterval(timer);
    };
  },
  now: () => Date.now(),
  ask: (records) => versionsOf(api.client, records),
};

/**
 * Until live updates arrive (ISS-14), the records on the screen are checked every 30 seconds and
 * when the person comes back to the tab (ADR-0019): one small request asks for all their versions.
 * `onNewer` is told which records somebody else changed since; it then reads them again.
 */
export function useVersionWatch(
  watched: readonly Watched[],
  onNewer: (newer: readonly Watched[]) => void,
): void {
  // The newest function is called; a new one does not restart the timer or the listeners.
  const told = useRef(onNewer);
  useEffect(() => {
    told.current = onNewer;
  }, [onNewer]);
  useEffect(
    () =>
      startWatching(browser, watched, watchSeconds, (newer) => {
        told.current(newer);
      }),
    [watched],
  );
}

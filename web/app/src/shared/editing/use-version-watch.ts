import { versionsOf, watchSeconds } from "@bugaboo/api-client";
import { useEffect, useRef } from "react";

import { api } from "../api.ts";
import { checkedJustNow, newerThan, type Found, type Watched } from "./version-watch.ts";

/** The events of the window after which the screen asks at once: the person comes back. */
const comingBack = ["focus", "online"] as const;

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
  useEffect(() => {
    let lastCheck = Number.NEGATIVE_INFINITY;
    const check = (): void => {
      if (watched.length === 0 || document.visibilityState === "hidden") {
        return;
      }
      if (checkedJustNow(lastCheck, Date.now())) {
        return;
      }
      lastCheck = Date.now();
      void versionsOf(
        api.client,
        watched.map((one) => one.record),
      ).then(
        (found: readonly Found[]) => {
          const newer = newerThan(watched, found);
          if (newer.length > 0) {
            told.current(newer);
          }
        },
        () => {
          // No answer now (offline, signed out): the next check tries again.
        },
      );
    };
    const timer = watched.length === 0 ? null : setInterval(check, watchSeconds * 1000);
    for (const name of comingBack) {
      window.addEventListener(name, check);
    }
    document.addEventListener("visibilitychange", check);
    return () => {
      if (timer !== null) {
        clearInterval(timer);
      }
      for (const name of comingBack) {
        window.removeEventListener(name, check);
      }
      document.removeEventListener("visibilitychange", check);
    };
  }, [watched]);
}

import { useEffect, useState } from "react";

/** How long nothing shows while content is on its way, so quick answers cause no flicker (§6.1). */
export const skeletonDelayMs = 300;

/** Starts a timer: how a clock is passed in, so tests can use their own. */
export type Schedule = (done: () => void, ms: number) => () => void;

/** The timers of the browser, as a `Schedule`. */
export const browserSchedule: Schedule = (done, ms) => {
  const timer = setTimeout(done, ms);
  return () => {
    clearTimeout(timer);
  };
};

/** Calls `onElapsed` after `ms` unless the returned function stops it first. */
export function startDelay(
  ms: number,
  onElapsed: () => void,
  schedule: Schedule = browserSchedule,
): () => void {
  return schedule(onElapsed, ms);
}

/** The delay that waits for nothing: used while there is nothing to wait for. */
const noDelay = (): void => {
  // Nothing was started, so nothing needs stopping.
};

/**
 * Whether something that is still loading has been loading for `ms` (0.3 s by default). Skeleton
 * shapes show only then, so a quick answer never flashes grey shapes.
 */
export function useAfterDelay(active: boolean, ms: number = skeletonDelayMs): boolean {
  const [elapsed, setElapsed] = useState(false);
  useEffect(() => {
    const stop = active
      ? startDelay(ms, () => {
          setElapsed(true);
        })
      : noDelay;
    return () => {
      stop();
      setElapsed(false);
    };
  }, [active, ms]);
  return active && elapsed;
}

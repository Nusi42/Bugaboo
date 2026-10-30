/**
 * How long the page counts as leaving after the browser asked whether to leave it. A leaving page does not
 * answer for the loads the browser cancels, which follow within a moment; a download that is started by
 * leaving to its address (the page stays) must not hide a failed load for long.
 */
const LEAVING_FOR_MS = 2000;

/**
 * The words with which the three engines say that a part of the app could not be loaded (Chrome, Firefox,
 * Safari). The list is the one of `isModuleNotFoundError` in `@tanstack/router-core`, which the router uses
 * to tell a failed load from another error.
 */
const MODULE_LOAD_FAILED = [
  "Failed to fetch dynamically imported module",
  "error loading dynamically imported module",
  "Importing a module script failed",
] as const;

/** Whether an error says that a part of the app could not be loaded. */
export function isModuleLoadError(error: unknown): boolean {
  return (
    error instanceof Error && MODULE_LOAD_FAILED.some((start) => error.message.startsWith(start))
  );
}

/**
 * Whether an error is reported as a defect. Every error is, but a failed load of a part of the app while
 * the page is being left: the browser cancels what is still loading, and nobody sees the page it ends.
 */
export function shouldReport(error: unknown, leaving: boolean): boolean {
  return !(leaving && isModuleLoadError(error));
}

let leaving = false;
let staying: number | undefined;

/** What the browser tells about a page that is hidden or shown again. */
export interface PageChange {
  readonly persisted: boolean;
}

/** The events of a window that say the page is going away or is back. */
export interface LeavingTarget {
  readonly addEventListener: (
    type: "beforeunload" | "pagehide" | "pageshow",
    listener: (event: PageChange) => void,
  ) => void;
}

/** The clock the page keeps its one timer by. */
export interface Clock {
  readonly after: (run: () => void, ms: number) => number;
  readonly cancel: (timer: number) => void;
}

/**
 * Notes when the page is being left (a reload, a link to another address, closing it). It is leaving from
 * `beforeunload` for a moment (the person may stay) and from `pagehide` for good, unless the browser keeps
 * the page for the way back (`persisted`), where `pageshow` ends it.
 */
export function watchLeavingPage(target: LeavingTarget, clock: Clock): void {
  target.addEventListener("beforeunload", () => {
    leaving = true;
    if (staying !== undefined) {
      clock.cancel(staying);
    }
    staying = clock.after(() => {
      leaving = false;
      staying = undefined;
    }, LEAVING_FOR_MS);
  });
  target.addEventListener("pagehide", (event) => {
    if (!event.persisted) {
      leaving = true;
    }
  });
  target.addEventListener("pageshow", () => {
    leaving = false;
  });
}

/** Whether the page is being left now. */
export function isLeavingPage(): boolean {
  return leaving;
}

/** Forgets what was noted (for a test that starts again). */
export function resetLeavingPage(): void {
  leaving = false;
  staying = undefined;
}

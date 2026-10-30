/** How long the page counts as leaving after a person was asked about leaving it and stayed (ms). */
const STAYED_AFTER_MS = 2000;

let leaving = false;

/** The events of a window that say the page is going away. */
interface LeavingTarget {
  readonly addEventListener: (type: "beforeunload" | "pagehide", listener: () => void) => void;
}

/**
 * Notes when the page is being left (a reload, a link to another address, closing it): from then on
 * the browser cancels what is still loading. Such a cancelled load (a part of the app that was
 * being fetched) is no defect of the app, and nobody sees the page it ends.
 */
export function watchLeavingPage(
  target: LeavingTarget,
  later: (restore: () => void, after: number) => void,
): void {
  target.addEventListener("beforeunload", () => {
    leaving = true;
    // If the person stays, the page is not leaving after all.
    later(() => {
      leaving = false;
    }, STAYED_AFTER_MS);
  });
  target.addEventListener("pagehide", () => {
    leaving = true;
  });
}

/** Whether the page is being left now. */
export function isLeavingPage(): boolean {
  return leaving;
}

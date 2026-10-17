import { useEffect } from "react";

/**
 * A window that is closed or reloaded runs no cleanup of its screen, so an edit lock is let go as
 * the page is hidden (the release is a `keepalive` request that the browser still sends, M3-F105,
 * M4-F26). A page that is put in the back and forward cache is hidden the same way and may be shown
 * again with its screen as it was, but without the lock it let go: `comeBack` then says so, so the
 * editor takes the lock again or tells the person that it is lost, and no text is lost.
 *
 * Listens on `target` (the window) and returns the way to stop; the one place that knows about
 * the events, for every editor that holds a lock.
 */
export function letGoWhenHidden(
  target: Readonly<EventTarget>,
  letGo: () => void,
  comeBack: () => void,
): () => void {
  const hidden = (): void => {
    letGo();
  };
  // What a `pageshow` carries is read as it comes: `persisted` says the page was in the cache.
  const shown = (event: unknown): void => {
    if (typeof event === "object" && event !== null && Reflect.get(event, "persisted") === true) {
      comeBack();
    }
  };
  target.addEventListener("pagehide", hidden);
  target.addEventListener("pageshow", shown);
  return () => {
    target.removeEventListener("pagehide", hidden);
    target.removeEventListener("pageshow", shown);
  };
}

/**
 * Lets the lock go when the page is hidden and when the screen goes away, and says when the page
 * comes back from the cache (see [`letGoWhenHidden`]). The functions must stay the same while the
 * screen is on.
 */
export function useLetGoWhenHidden(letGo: () => void, comeBack: () => void): void {
  useEffect(() => {
    const stop = letGoWhenHidden(window, letGo, comeBack);
    return () => {
      stop();
      letGo();
    };
  }, [letGo, comeBack]);
}

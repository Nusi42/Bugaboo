import { onlineManager } from "@tanstack/react-query";

/**
 * Waits for a read of the query cache, unless the browser is offline or goes offline meanwhile: the cache pauses a read
 * while offline, and what waits for it (an editor that closes onto the saved text) would wait until the connection
 * came back. The read itself goes on and is drawn when it answers.
 */
export function readUnlessOffline(read: () => Promise<void>): Promise<void> {
  if (!onlineManager.isOnline()) {
    return Promise.resolve();
  }
  return new Promise<void>((done) => {
    const stop = onlineManager.subscribe((online) => {
      if (!online) {
        stop();
        done();
      }
    });
    const finish = (): void => {
      stop();
      done();
    };
    // A failed read ends the wait as an answer does: what waits goes on, and nothing is left unhandled.
    read().then(finish, finish);
  });
}

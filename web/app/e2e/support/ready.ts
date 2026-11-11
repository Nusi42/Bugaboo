import { setTimeout as sleep } from "node:timers/promises";

import { exchange } from "./session-transport.ts";

/** How long a server gets to start before the test gives up. */
export const START_LIMIT_MS = 30_000;
/** The first pause between two questions; it doubles each time up to `LONGEST_PAUSE_MS`. */
export const FIRST_PAUSE_MS = 50;
export const LONGEST_PAUSE_MS = 1000;
/** The route of the server that answers when it can serve (M1 `/health/live`). */
export const LIVE_PATH = "/health/live";

/** What the wait needs to know about the program it waits for. */
export interface Waited {
  readonly exited: () => boolean;
  readonly output: () => string;
}

/** One question to the server: `true` when it answered that it is alive. */
export type Ask = () => Promise<boolean>;

/**
 * Asks the server over HTTPS whether it is alive, with the certificate it made itself. The question
 * goes through the pool of connections of the server (`session-transport.ts`), so the connection of
 * the answer is the one the sessions of the tests go on with: a question that is answered leaves
 * nothing closed behind (a closed connection stays in TIME_WAIT for two minutes, and every file of
 * tests starts a server), and one that is refused leaves nothing. `https: false` is for the unit
 * test, which has no certificate.
 */
export function askLive(port: number, https = true): Ask {
  const origin = `${https ? "https" : "http"}://localhost:${String(port)}`;
  return () =>
    exchange({ url: `${origin}${LIVE_PATH}`, method: "GET", headers: {}, body: "" }, origin).then(
      (answer) => answer.status === 200,
      () => false,
    );
}

/**
 * Waits until the server answers, with a longer pause after each question that is not answered
 * (never a question every few milliseconds, which would use up the ports of the computer, WP-03 of
 * M4). Gives up when the program ends, or when it takes longer than `limitMs`.
 */
export async function waitUntilReady(
  ask: Ask,
  server: Waited,
  limitMs = START_LIMIT_MS,
  started = Date.now(),
  pause = FIRST_PAUSE_MS,
): Promise<void> {
  if (server.exited()) {
    throw new Error(`The server stopped while starting:\n${server.output()}`);
  }
  if (Date.now() - started > limitMs) {
    throw new Error(`The server did not answer within ${String(limitMs)} ms:\n${server.output()}`);
  }
  if (!(await ask())) {
    await sleep(pause);
    await waitUntilReady(ask, server, limitMs, started, Math.min(pause * 2, LONGEST_PAUSE_MS));
  }
}

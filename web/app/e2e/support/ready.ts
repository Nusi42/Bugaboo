import { request as plain } from "node:http";
import { request as secure } from "node:https";
import { setTimeout as sleep } from "node:timers/promises";

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
 * Asks the server over HTTPS whether it is alive, with the certificate it made itself. One short
 * question, closed after the answer (`agent: false`), so a question that is answered leaves one
 * connection behind and one that is refused leaves none. `https: false` is for the unit test,
 * which has no certificate.
 */
export function askLive(port: number, https = true): Ask {
  const request = https ? secure : plain;
  return () =>
    new Promise((done) => {
      const question = request(
        {
          host: "localhost",
          port,
          path: LIVE_PATH,
          method: "GET",
          agent: false,
          rejectUnauthorized: false,
          headers: { connection: "close" },
        },
        (answer) => {
          answer.resume();
          done(answer.statusCode === 200);
        },
      );
      question.once("error", () => {
        done(false);
      });
      question.end();
    });
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

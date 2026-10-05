import { hashKey, type QueryClient, type QueryKey } from "@tanstack/react-query";

import type { SaveState } from "@bugaboo/ui";

/** What a query holds: an object or a list. */
export type Cacheable = object;

/** What a change is told: the values of one action. */
export type Described = object;

/**
 * A change that shows at once and is sent in the background (UX-06, UX §6). `apply` says what the
 * cached data looks like after the change; `send` asks the server. When the server refuses, the
 * data goes back to what it was, and the caller shows the plain message of the error.
 */
export interface OptimisticChange<Variables extends Described, Cached extends Cacheable, Answer> {
  /** The data on screen that the change touches. */
  readonly queryKey: QueryKey;
  /** Whether what the cache holds under the key is data of this kind (a guard, never a guess). */
  readonly isCached: (value: unknown) => value is Cached;
  /** The data after the change, worked out from the data before it. */
  readonly apply: (before: Cached | undefined, variables: Variables) => Cached | undefined;
  readonly send: (variables: Variables) => Promise<Answer>;
}

/** One change that is on its way: what it does to the data. */
interface Pending {
  readonly apply: (before: unknown) => unknown;
}

/**
 * The changes of one query that are on their way, and the data from before the first of them.
 * (A class: it has state, and its methods take no parameters the lint rules cannot make read-only.)
 */
class Flight {
  base: unknown = undefined;
  readonly changes = new Map<number, Pending>();

  /** The data from before the first change with every change that is still on its way put on it. */
  replay(): unknown {
    let data = this.base;
    for (const change of this.changes.values()) {
      data = change.apply(data);
    }
    return data;
  }
}

/** The changes on their way, by cache and query. A cache is the one object that is not a value. */
const flights = new WeakMap<QueryClient, Map<string, Flight>>();

let nextChange = 0;

/**
 * The function a TanStack Query mutation runs: it does the change in the cache first, asks the
 * server, and passes an error on so the caller can show its plain message. Changes may overlap: when
 * the server refuses one, the data is made again from the data before them with the changes that
 * are still on their way put back on it, so a refusal never takes another change with it. The
 * data is read from the server again once, after the last change on its way, and nobody waits for
 * that: *Saved* comes as soon as the server has answered.
 *
 * The client comes as a function because the cache is the one object that is not a value.
 */
export function optimisticSend<Variables extends Described, Cached extends Cacheable, Answer>(
  getClient: () => QueryClient,
  change: OptimisticChange<Variables, Cached, Answer>,
): (variables: Variables) => Promise<Answer> {
  return async (variables) => {
    const client = getClient();
    const hash = hashKey(change.queryKey);
    // An answer that is still on its way would put the old data back over the change.
    await client.cancelQueries({ queryKey: change.queryKey });
    const all = flights.get(client) ?? new Map<string, Flight>();
    flights.set(client, all);
    const flight = all.get(hash) ?? new Flight();
    all.set(hash, flight);
    if (flight.changes.size === 0) {
      flight.base = client.getQueryData(change.queryKey);
    }
    const id = nextChange;
    nextChange += 1;
    // What this change does to whatever data it is put on.
    const pending: Pending = {
      apply: (before) => change.apply(change.isCached(before) ? before : undefined, variables),
    };
    flight.changes.set(id, pending);
    client.setQueryData(change.queryKey, pending.apply(client.getQueryData(change.queryKey)));
    try {
      const answer = await change.send(variables);
      // The server has it now: from here on it is part of the data before the others.
      flight.base = pending.apply(flight.base);
      return answer;
    } catch (error) {
      flight.changes.delete(id);
      client.setQueryData(change.queryKey, flight.replay());
      throw error;
    } finally {
      flight.changes.delete(id);
      if (flight.changes.size === 0) {
        all.delete(hash);
        void client.invalidateQueries({ queryKey: change.queryKey });
      }
    }
  };
}

/** What a mutation has gone through, as the words *Saving…*, *Saved* and *Not saved yet* say it. */
const saveStates = {
  idle: "idle",
  pending: "saving",
  success: "saved",
  error: "not-saved",
} as const satisfies Record<string, SaveState>;

/** What a mutation has gone through, as the words *Saving…*, *Saved* and *Not saved yet* say it. */
export function saveStateOf(status: keyof typeof saveStates): SaveState {
  return saveStates[status];
}

/** What the hook knows of one call of a change. */
export interface Call {
  readonly status: keyof typeof saveStates;
  readonly error: unknown;
}

/** Where a run of overlapping changes stands: what to say, and the error to show, if any. */
export interface Summary {
  readonly saveState: SaveState;
  /** The error of the latest change that was refused, or `null`. */
  readonly error: unknown;
}

/**
 * The state of the changes since everything was last idle (the calls of one burst). While one is
 * on its way: saving. When all are done: not saved yet if any was refused, whatever the later ones
 * did, and the error of the latest refusal is shown; saved only when none was refused.
 */
export function summarizeCalls(calls: readonly Call[]): Summary {
  let refused: Call | undefined;
  let waiting = false;
  for (const call of calls) {
    waiting = waiting || call.status === "pending";
    refused = call.status === "error" ? call : refused;
  }
  if (waiting) {
    return { saveState: saveStateOf("pending"), error: null };
  }
  if (refused !== undefined) {
    return { saveState: saveStateOf("error"), error: refused.error };
  }
  return { saveState: saveStateOf(calls.at(-1)?.status ?? "idle"), error: null };
}

/** How long nothing shows while content is on its way, so quick answers cause no flicker (§6.1). */
export const skeletonDelayMs = 300;

/** How long *Undo* stays on offer after a reversible action (UX §6). */
export const undoWindowMs = 10_000;

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

import type { QueryClient, QueryKey } from "@tanstack/react-query";

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
  /** The data after the change, worked out from the data before it. */
  readonly apply: (before: Cached | undefined, variables: Variables) => Cached | undefined;
  readonly send: (variables: Variables) => Promise<Answer>;
}

/**
 * The function a TanStack Query mutation runs: it does the change in the cache first, asks the
 * server, puts the data back when the server refuses (and passes the error on, so the caller can
 * show its plain message), and reads the data again when it is done, so the screen ends up
 * showing what the server says either way.
 *
 * The client comes as a function because the cache is the one object that is not a value.
 */
export function optimisticSend<Variables extends Described, Cached extends Cacheable, Answer>(
  getClient: () => QueryClient,
  change: OptimisticChange<Variables, Cached, Answer>,
): (variables: Variables) => Promise<Answer> {
  return async (variables) => {
    const client = getClient();
    // An answer that is still on its way would put the old data back over the change.
    await client.cancelQueries({ queryKey: change.queryKey });
    const before = client.getQueryData<Cached>(change.queryKey);
    client.setQueryData<Cached | undefined>(change.queryKey, change.apply(before, variables));
    try {
      return await change.send(variables);
    } catch (error) {
      client.setQueryData<Cached | undefined>(change.queryKey, before);
      throw error;
    } finally {
      await client.invalidateQueries({ queryKey: change.queryKey });
    }
  };
}

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

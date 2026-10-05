import { useMutation, useMutationState, useQueryClient } from "@tanstack/react-query";
import { useEffect, useId, useState, useSyncExternalStore } from "react";

import { messageOf } from "./errors.ts";
import { removeKeepingFocus } from "./focus.ts";
import {
  optimisticSend,
  saveStateOf,
  skeletonDelayMs,
  startDelay,
  type Cacheable,
  type Described,
  type OptimisticChange,
} from "./optimistic.ts";
import { undoStore, type UndoOffer } from "./optimistic-undo.ts";

import type { SaveState } from "@bugaboo/ui";

export interface OptimisticMutation<Variables> {
  /** Shows the change now and sends it. */
  readonly change: (variables: Variables) => void;
  /**
   * Like `change`, for a row that goes away: the focus moves to the next row, or the one before it,
   * or the heading of the list, so a keyboard or screen reader user is not left on nothing.
   */
  readonly remove: (variables: Variables) => void;
  /** *Saving…*, *Saved* or *Not saved yet*; nothing before the first change. */
  readonly saveState: SaveState;
  /** The plain message when the server refused, else `null`; the screen already shows the old data. */
  readonly problem: string | null;
}

/**
 * A change that shows at once (UX-06): the data on screen changes before the server answers, goes
 * back with a plain message when the server refuses, and `saveState` says where things stand.
 */
export function useOptimisticMutation<
  Variables extends Described,
  Cached extends Cacheable,
  Answer,
>(change: OptimisticChange<Variables, Cached, Answer>): OptimisticMutation<Variables> {
  const client = useQueryClient();
  // Every call is a mutation of its own with this key, so changes that overlap each have a state.
  const mutationKey = ["optimistic", useId()];
  const mutation = useMutation({
    mutationKey,
    mutationFn: optimisticSend(() => client, change),
  });
  const calls = useMutationState({ filters: { mutationKey } });
  const last = calls.at(-1);
  let waiting = false;
  for (const call of calls) {
    waiting = waiting || call.status === "pending";
  }
  const run = (variables: Variables): void => {
    mutation.mutate(variables);
  };
  return {
    change: run,
    remove: (variables) => {
      removeKeepingFocus(() => {
        run(variables);
      });
    },
    saveState: waiting ? saveStateOf("pending") : saveStateOf(last?.status ?? "idle"),
    problem: !waiting && last?.status === "error" ? messageOf(last.error) : null,
  };
}

/**
 * Whether something that is still loading has been loading for `ms` (0.3 s by default). Skeleton
 * shapes show only then, so a quick answer never flashes grey shapes.
 */
/** The delay that waits for nothing: used while there is nothing to wait for. */
const noDelay = (): void => {
  // Nothing was started, so nothing needs stopping.
};

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

/** The actions that can still be taken back, for the place that shows *Undo*. */
export function useUndoOffers(): readonly UndoOffer[] {
  return useSyncExternalStore(undoStore.subscribe, undoStore.list);
}

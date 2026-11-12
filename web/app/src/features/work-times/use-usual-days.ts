import { type Schema, dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useQueryClient } from "@tanstack/react-query";
import { useRef } from "react";

import { api } from "../../shared/api.ts";
import { doNothing } from "../../shared/do-nothing.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import {
  isWorkTimes,
  withDay,
  type UsualDay,
  type WorkTimes,
  type WorkTimesRead,
  type Whose,
} from "./work-times.ts";

/**
 * A change of the usual work days: the seven days after it, the version of the usual work days the
 * person looked at when they made it, and the days to go back to with *Undo*.
 */
export interface UsualChange {
  readonly days: readonly UsualDay[];
  readonly version: number;
  /** The day that changed, as it was before: what *Undo* puts back. None for the change that *Undo* makes. */
  readonly before?: { readonly at: number; readonly day: UsualDay };
}

/** Sends the seven days on a version of the usual work days. */
async function sendDays(
  whose: Whose,
  version: number,
  days: readonly UsualDay[],
): Promise<Schema<"UsualDaysBody">> {
  const body = { version, days: [...days] };
  return dataOf(
    whose === undefined
      ? await api.client.PUT("/api/v1/me/work-times/usual-days", { body })
      : await api.client.PUT("/api/v1/people/{person}/work-times/usual-days", {
          params: { path: { person: whose } },
          body,
        }),
  );
}

/**
 * The version a change is sent on: the one it was made on, moved on along this page's own changes
 * that were answered since (a change made before the page read the answer of the one before it). A
 * change by anybody else in between is not on that path, so the server refuses the change as made
 * on an old version instead of it overwriting the other.
 */
function sentOn(movedTo: (version: number) => number | undefined, version: number): number {
  let current = version;
  for (let next = movedTo(current); next !== undefined; next = movedTo(current)) {
    current = next;
  }
  return current;
}

/**
 * The changes of the usual work days and work hours of one person: each shows at once and saves in
 * place with *Undo* (R7). The changes go to the server one after the other. A page of another person
 * starts with nothing remembered: the component is keyed by whose days it shows.
 */
export function useUsualDays(whose: Whose, read: WorkTimesRead): OptimisticMutation<UsualChange> {
  // The versions this page's own changes moved the usual work days from and to.
  const made = useRef(new Map<number, number>());
  const client = useQueryClient();
  const line = useRef<Promise<unknown>>(Promise.resolve());
  const edits = useOptimisticMutation<UsualChange, WorkTimes, Schema<"UsualDaysBody">>({
    queryKey: read.comingKey,
    isCached: isWorkTimes,
    apply: (before, change) =>
      before === undefined ? undefined : { ...before, usual: [...change.days] },
    send: (change) => {
      const sent = line.current.then(async () => {
        const version = sentOn((from) => made.current.get(from), change.version);
        const answer = await sendDays(whose, version, change.days);
        made.current.set(version, answer.version);
        const before = change.before;
        if (before !== undefined) {
          undoStore.offer({
            message: t("work-times.usual-changed"),
            // Only the day this change touched goes back, on the days as they are now: a later
            // change of another day stays.
            undo: () => {
              const cached: unknown = client.getQueryData(read.comingKey);
              const now = isWorkTimes(cached) ? cached.usual : change.days;
              edits.change({ days: withDay(now, before.at, before.day), version: answer.version });
            },
          });
        }
        return answer;
      });
      line.current = sent.catch(doNothing);
      return sent;
    },
  });
  return edits;
}

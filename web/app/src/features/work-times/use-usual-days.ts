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
  type UsualDay,
  type WorkTimes,
  type WorkTimesRead,
  type Whose,
} from "./work-times.ts";

/** A change of the usual work days: the seven days after it, and the days to go back to with *Undo*. */
export interface UsualChange {
  readonly days: readonly UsualDay[];
  /** The days before the change; none for the change that *Undo* makes. */
  readonly before?: readonly UsualDay[];
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
 * The changes of the usual work days and work hours: each shows at once and saves in place with
 * *Undo* (R7). A change is made on the version that was looked at, so the changes go to the server
 * one after the other, each on the version the one before it made.
 */
export function useUsualDays(whose: Whose, read: WorkTimesRead): OptimisticMutation<UsualChange> {
  const client = useQueryClient();
  const known = useRef<number>(0);
  const line = useRef<Promise<unknown>>(Promise.resolve());
  const edits = useOptimisticMutation<UsualChange, WorkTimes, Schema<"UsualDaysBody">>({
    queryKey: read.comingKey,
    isCached: isWorkTimes,
    apply: (before, change) =>
      before === undefined ? undefined : { ...before, usual: [...change.days] },
    send: (change) => {
      const sent = line.current.then(async () => {
        const cached: unknown = client.getQueryData(read.comingKey);
        const version = Math.max(known.current, isWorkTimes(cached) ? cached.usual_version : 0);
        const answer = await sendDays(whose, version, change.days);
        known.current = answer.version;
        const before = change.before;
        if (before !== undefined) {
          undoStore.offer({
            message: t("work-times.usual-changed"),
            undo: () => {
              edits.change({ days: before });
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

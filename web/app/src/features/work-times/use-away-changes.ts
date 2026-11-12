import { dataOf } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { removeKeepingFocus } from "../../shared/focus.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshWorkTimes } from "../../shared/work-times-reads.ts";
import type { Away, AwayDays, Whose } from "./work-times.ts";
import { awayDaysText } from "./work-times-words.ts";

/** Enters an away time for `whose`. */
async function enter(whose: Whose, days: AwayDays): Promise<Away> {
  return dataOf(
    whose === undefined
      ? await api.client.POST("/api/v1/me/away-times", { body: days })
      : await api.client.POST("/api/v1/people/{person}/away-times", {
          params: { path: { person: whose } },
          body: days,
        }),
  );
}

/** Changes the days of an away time, on the version that was looked at. */
async function change(away: Away, days: AwayDays): Promise<Away> {
  return dataOf(
    await api.client.PATCH("/api/v1/away-times/{id}", {
      params: { path: { id: away.id } },
      body: { version: away.version, days },
    }),
  );
}

/** Removes an away time, on the version that was looked at. */
async function remove(away: Away): Promise<void> {
  await api.client.DELETE("/api/v1/away-times/{id}", {
    params: { path: { id: away.id }, query: { version: away.version } },
  });
}

/** The days of an away time as a request names them again: its note only when the reader has it. */
export function daysOf(away: Away): AwayDays {
  return {
    first: away.first,
    last: away.last,
    first_afternoon_only: away.first_afternoon_only,
    last_morning_only: away.last_morning_only,
    ...(away.note === null ? {} : { note: away.note }),
  };
}

/** What the page of work times does with away times. */
export interface AwayChanges {
  /** Enters an away time, or changes the one given; resolves to the plain message of a refusal, or `null`. */
  readonly save: (days: AwayDays, changing: Away | undefined) => Promise<string | null>;
  /** Deletes an away time at once, with *Undo* (UX §6). */
  readonly remove: (away: Away) => void;
  /** The plain message when the server refused a deletion or an *Undo*, else `null`. */
  readonly problem: string | null;
}

/**
 * Entering, changing and deleting away times of `whose` (PLAN-11). Each offers *Undo*, which does the
 * opposite; the server decides every time, and the page reads the work times again after each.
 */
export function useAwayChanges(whose: Whose): AwayChanges {
  const [problem, setProblem] = useState<string | null>(null);
  const run = (step: () => Promise<unknown>): void => {
    setProblem(null);
    step()
      .then(refreshWorkTimes)
      .catch((error: unknown) => {
        setProblem(messageOf(error));
      });
  };
  const offerBack = (message: string, back: () => Promise<unknown>): void => {
    undoStore.offer({
      message,
      undo: () => {
        run(back);
      },
    });
  };
  return {
    save: async (days, changing) => {
      try {
        const made =
          changing === undefined ? await enter(whose, days) : await change(changing, days);
        refreshWorkTimes();
        if (changing === undefined) {
          offerBack(t("work-times.away-added", { days: awayDaysText(made) }), () => remove(made));
        } else {
          offerBack(t("work-times.away-changed", { days: awayDaysText(made) }), () =>
            change(made, daysOf(changing)),
          );
        }
        return null;
      } catch (error) {
        return messageOf(error);
      }
    },
    remove: (away) => {
      removeKeepingFocus(() => {
        run(async () => {
          await remove(away);
          offerBack(t("work-times.away-deleted", { days: awayDaysText(away) }), () =>
            enter(whose, daysOf(away)),
          );
        });
      });
    },
    problem,
  };
}

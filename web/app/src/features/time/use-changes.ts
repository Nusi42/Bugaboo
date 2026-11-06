import { dataOf, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { deviceTimeZone } from "../../shared/region.ts";
import { codeOf, retryAfterSecondsOf } from "../../shared/errors.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { rereadTimesheet } from "../../shared/time-keys.ts";
import type { PlannedCell, Times } from "./edits.ts";

/** A change of the times of a work day, made on the version that was looked at. */
export interface DayChange {
  readonly date: string;
  readonly version: number;
  readonly times: Times;
  /** Why, when the change asks for one (a past day, or the day of somebody else). */
  readonly reason: string | undefined;
}

/** How often a change that the server turned away for coming too soon is tried again. */
const RETRIES = 3;
/** How long to wait when the server does not say: the changes of a person's days are taken one a second. */
const WAIT_SECONDS = 1;
const MS_PER_SECOND = 1000;

/** Tries a change again after the wait the server asks for, while it says `limit.reached`. */
async function untilTaken(work: () => Promise<unknown>, left = RETRIES): Promise<void> {
  try {
    await work();
  } catch (error) {
    if (left === 0 || codeOf(error) !== "limit.reached") {
      throw error;
    }
    const seconds = retryAfterSecondsOf(error) ?? WAIT_SECONDS;
    await new Promise((resolve) => {
      setTimeout(resolve, seconds * MS_PER_SECOND);
    });
    await untilTaken(work, left - 1);
  }
}

/**
 * Saves the times of a work day: one's own, or another person's for a holder of *Change working
 * hours of others*. Whatever is saved or refused, the week is read again before it answers, so the
 * next change is made on what the server has.
 */
export function useDayChange(person: string | undefined) {
  const own = api.query.useMutation("patch", "/api/v1/me/work-days/{date}");
  const other = api.query.useMutation("patch", "/api/v1/people/{person}/work-days/{date}");
  const save = async (change: DayChange): Promise<void> => {
    const body = {
      version: change.version,
      periods: [...change.times.periods],
      breaks: [...change.times.breaks],
      reason: change.reason ?? null,
      // Counts only for a date with no record, when the person and the organization chose no zone.
      device_zone: deviceTimeZone(),
    };
    try {
      await untilTaken(() =>
        person === undefined
          ? own.mutateAsync({ params: { path: { date: change.date } }, body })
          : other.mutateAsync({ params: { path: { person, date: change.date } }, body }),
      );
    } finally {
      await rereadTimesheet();
    }
  };
  return { save };
}

/**
 * What a change of a time entry renews, once, wherever it is made (a cell of the timesheet, the
 * details of a row): the week is read again and answers when it is read, so the next change is
 * planned on what the server has; the time of the issue and its history are read again too.
 */
async function settledEntries(): Promise<void> {
  refreshIssues();
  refreshHistory();
  await rereadTimesheet();
}

/** A change of one of the person's own entries, made on the version that was looked at. */
export interface EntryChange {
  readonly entryId: string;
  readonly version: number;
  readonly minutes: number;
  /** The note, *Billable* and fields, when they change too. */
  readonly entry?: Schema<"EntryDetailsBody">;
}

/** Changes one of the person's own entries (its length, and its note, *Billable* and fields). */
export async function changeEntry(change: EntryChange): Promise<void> {
  try {
    dataOf(
      await api.client.PATCH("/api/v1/me/time-entries/{entry}", {
        params: { path: { entry: change.entryId } },
        body: {
          version: change.version,
          minutes: change.minutes,
          ...(change.entry === undefined ? {} : { entry: change.entry }),
        },
      }),
    );
  } finally {
    await settledEntries();
  }
}

/** Books, changes or takes back the time of a cell of the person's own week. */
export async function applyEntryChange(planned: PlannedCell): Promise<void> {
  try {
    if (planned.kind === "add") {
      dataOf(
        await api.client.POST("/api/v1/me/time-entries", {
          body: { issue: planned.issue, date: planned.date, minutes: planned.minutes },
        }),
      );
    } else if (planned.kind === "change") {
      await changeEntry({
        entryId: planned.entry,
        version: planned.version,
        minutes: planned.minutes,
      });
    } else if (planned.kind === "delete") {
      dataOf(
        await api.client.DELETE("/api/v1/me/time-entries/{entry}", {
          params: { path: { entry: planned.entry }, query: { version: planned.version } },
        }),
      );
    }
  } finally {
    // The change of a length settled itself (`changeEntry`); the others do now.
    if (planned.kind !== "change") {
      await settledEntries();
    }
  }
}

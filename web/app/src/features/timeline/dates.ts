import { t } from "@bugaboo/messages";

import { messageOf } from "../../shared/errors.ts";
import { formatCalendarDay } from "../../shared/format.ts";
import { inOrder } from "../../shared/in-order.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { sendChange } from "../issues/index.ts";
import { changeOf, type DateFields, type KeptDates } from "./date-fields.ts";
import type { BarDays } from "./lines.ts";
import { idOf, refreshTimeline, type TimelineRow } from "./timeline-data.ts";

export { moved, shownWhileMoving, type Move } from "./moves.ts";

/**
 * The dates a bar is saved with (PLAN-01, PLAN-03, ADR-0043): the two date fields of the view,
 * *Start date* and *Due date* or two own date fields, changed on the issue like any change of it, one
 * after the other, each on the version the one before made, with *Undo*. The server judges every
 * change and works out the bars again; the screen only says which dates the person moved.
 */

/** The days of a bar in words: "3 Mar 2027 to 5 Mar 2027". */
export function daysText(days: BarDays): string {
  return t("timeline.days", {
    first: formatCalendarDay(days.first),
    last: formatCalendarDay(days.last),
  });
}

/**
 * The dates a change saved, in words: both when both are kept, which says so also of a day that
 * *Plan by effort* worked out and that a moved end now keeps (M8-F83).
 */
function savedText(dates: KeptDates): string {
  if (dates.start !== null && dates.end !== null) {
    return t("timeline.saved-both", {
      start: formatCalendarDay(dates.start),
      end: formatCalendarDay(dates.end),
    });
  }
  return dates.start === null
    ? t("timeline.saved-end", { end: formatCalendarDay(dates.end ?? "") })
    : t("timeline.saved-start", { start: formatCalendarDay(dates.start) });
}

const daysPerWeek = 7;

/** "WEB-280 moved 1 week later, now from … to …", "… 3 days earlier". */
function movedText(key: string, days: number, dates: KeptDates): string {
  const count = Math.abs(days);
  const amount =
    count % daysPerWeek === 0
      ? t("timeline.amount-weeks", { count: count / daysPerWeek })
      : t("timeline.amount-days", { count });
  return t(days > 0 ? "timeline.moved-later" : "timeline.moved-earlier", {
    key,
    amount,
    dates: savedText(dates),
  });
}

/** The newest version of each issue that a save of this screen made. */
const savedVersions = new Map<string, number>();

/** The version a change is made on: what the page read, or what the last save made, whichever is newer. */
function versionNow(row: TimelineRow): number {
  const id = idOf(row);
  return Math.max(row.issue.version ?? 0, savedVersions.get(id) ?? 0);
}

/**
 * Saves new dates for the issue of a row, after every save of it asked for before, on the version
 * the one before made. A refusal comes back as the server's reason, in words.
 */
function save(row: TimelineRow, fields: DateFields, dates: KeptDates): Promise<string | null> {
  const id = idOf(row);
  return inOrder(`timeline:${id}`, async () => {
    try {
      const version = await sendChange(id, {
        version: versionNow(row),
        values: changeOf(fields, dates),
        // A run of keys is saved as the page is hidden; the browser still sends it (`use-key-runs.ts`).
        keepalive: true,
      });
      if (version !== undefined) {
        savedVersions.set(id, version);
      }
      return null;
    } catch (error) {
      return messageOf(error);
    }
  });
}

/**
 * Runs `work` on an issue after every save of it that this screen asked for before, with the
 * version the last of them made: a way out of a plan warning sent right after a drag is made on the
 * dates the drag saved, never on the version the page read.
 */
export function afterSaves<T>(row: TimelineRow, work: (version: number) => Promise<T>): Promise<T> {
  return inOrder(`timeline:${idOf(row)}`, () => work(versionNow(row)));
}

/**
 * Runs a change of an issue after every save of it that this screen asked for, like `afterSaves`,
 * and keeps the version it made, so the next change of the screen is made on that one.
 */
export async function changeAfterSaves(
  row: TimelineRow,
  change: (version: number) => Promise<number | undefined>,
): Promise<void> {
  const made = await afterSaves(row, change);
  if (made !== undefined) {
    savedVersions.set(idOf(row), made);
  }
}

/** What a change of dates says when it is done: by how many days it moved, or nothing more. */
export type Said = { readonly move: "both"; readonly days: number } | { readonly move: "set" };

/** What a change of dates says once it is saved: what happened and the dates it saved. */
export function changedText(row: TimelineRow, said: Said, dates: KeptDates): string {
  const key = known(row.issue.readable_id) ?? "";
  return said.move === "both"
    ? movedText(key, said.days, dates)
    : t("timeline.dates-changed", { key, dates: savedText(dates) });
}

/**
 * Saves new dates of a bar, after every change of it asked for before, and reads the timeline again
 * either way, so it shows the plan the server answers. Answers the server's reason when it refused.
 * Who offers *Undo* is the one who knows which change is the newest (`use-saving.ts`).
 */
export async function changeDates(
  row: TimelineRow,
  fields: DateFields,
  dates: KeptDates,
): Promise<string | null> {
  const refused = await save(row, fields, dates);
  await refreshTimeline();
  return refused;
}

/** Says why a bar cannot be moved. */
export function sayLocked(row: TimelineRow): void {
  undoStore.offer({ message: t("timeline.locked", { key: known(row.issue.readable_id) ?? "" }) });
}

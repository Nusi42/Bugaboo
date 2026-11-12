import { t } from "@bugaboo/messages";

import { formatCalendarDay } from "../../shared/format.ts";
import { inOrder } from "../../shared/in-order.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { refreshTimeline } from "./timeline-data.ts";
import { sendChange } from "../issues/index.ts";
import { changeOf, keptDates, type DateFields, type KeptDates } from "./date-fields.ts";
import type { BarDays } from "./lines.ts";

export { moved, shownWhileMoving, type Move } from "./moves.ts";
import type { TimelineRow } from "./timeline-data.ts";

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

const daysPerWeek = 7;

/** "WEB-280 moved 1 week later", "… 3 days earlier". */
function movedText(key: string, days: number): string {
  const count = Math.abs(days);
  const amount =
    count % daysPerWeek === 0
      ? t("timeline.amount-weeks", { count: count / daysPerWeek })
      : t("timeline.amount-days", { count });
  return t(days > 0 ? "timeline.moved-later" : "timeline.moved-earlier", { key, amount });
}

/** The newest version of each issue that a save of this screen made. */
const savedVersions = new Map<string, number>();

/** The version a change is made on: what the page read, or what the last save made, whichever is newer. */
function versionNow(row: TimelineRow): number {
  const id = known(row.issue.id) ?? "";
  return Math.max(row.issue.version ?? 0, savedVersions.get(id) ?? 0);
}

/**
 * Saves new dates for the issue of a row, after every save of it asked for before, on the version
 * the one before made. Answers whether the server took them.
 */
function save(row: TimelineRow, fields: DateFields, dates: KeptDates): Promise<boolean> {
  const id = known(row.issue.id) ?? "";
  return inOrder(`timeline:${id}`, async () => {
    const version = await sendChange(id, {
      version: versionNow(row),
      values: changeOf(fields, dates),
    });
    if (version !== undefined) {
      savedVersions.set(id, version);
    }
    return version !== undefined;
  });
}

/** What a change of dates says when it is done: what happened, in a sentence. */
export type Said =
  | { readonly move: "both"; readonly days: number }
  | { readonly move: "set"; readonly days: BarDays };

/**
 * Saves the new dates of a bar and offers *Undo*, which puts back the dates it kept before. A save
 * the server refuses is said, and the timeline is read again either way, so it shows the plan the
 * server answers.
 */
export async function changeDates(
  row: TimelineRow,
  fields: DateFields,
  dates: KeptDates,
  said: Said,
): Promise<void> {
  const key = known(row.issue.readable_id) ?? "";
  const before = keptDates(row, fields);
  const saved = await save(row, fields, dates);
  await refreshTimeline();
  if (!saved) {
    undoStore.offer({ message: t("timeline.not-saved", { key }) });
    return;
  }
  undoStore.offer({
    message:
      said.move === "both"
        ? movedText(key, said.days)
        : t("timeline.dates-changed", { key, days: daysText(said.days) }),
    undo: () => {
      void save(row, fields, before).then(refreshTimeline);
    },
  });
}

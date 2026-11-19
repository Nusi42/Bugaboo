import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "../../app/e2e/support/api.ts";
import {
  type KnownIssue,
  type KnownRecords,
  type KnownSpan,
  type KnownWorkDay,
  momentOf,
} from "./known.ts";

// The known records of the upgrade test (`known.ts`), found by their keys in what the API reads
// after the upgrade (architecture §11, "The upgrade test"): the issues by title, the comments and
// the time entries of an issue by its title, and a work day by the person and the day.

type Client = ApiSession["client"];

/** An issue as the list reads it: its readable ID, its title and the key of its status. */
export interface IssueRow {
  readonly key: string;
  readonly title: string;
  readonly status: string;
}

/** A time entry as the upgrade test compares it. */
export interface EntryRow {
  readonly date: string;
  readonly minutes: number;
}

/** The stretches of the work day of one person on one day, as the known work day names them. */
export async function workDayOf(
  client: Client,
  person: string | undefined,
  date: string,
): Promise<readonly KnownSpan[]> {
  if (person === undefined) {
    return [];
  }
  const read = dataOf(
    await client.GET("/api/v1/people/{person}/work-days", {
      params: { path: { person }, query: { from: date, to: date } },
    }),
  );
  const spans: KnownSpan[] = [];
  for (const day of read.days) {
    for (const period of day.date === date ? day.periods : []) {
      spans.push({ start: momentOf(period.start), end: momentOf(period.end) });
    }
  }
  return spans;
}

/** What the API reads of the issues, by issue: the comments and the time entries of each. */
export interface IssuesRead {
  readonly rows: readonly IssueRow[];
  readonly comments: readonly (readonly string[])[];
  readonly entries: readonly (readonly EntryRow[])[];
}

/** The known records the dump names, each found by its key in what the API read. */
export async function knownRead(
  { rows, comments, entries }: IssuesRead,
  known: KnownRecords,
  workDay: (day: KnownWorkDay) => Promise<readonly KnownSpan[]>,
): Promise<KnownRecords> {
  const at = (title: string): number => rows.findIndex((row) => row.title === title);
  const issue = ({ title }: KnownIssue): KnownIssue => ({
    title,
    status: rows[at(title)]?.status ?? "",
  });
  const { issue: commented } = known.comments;
  const { issue: entered, date } = known.timeEntries;
  return {
    issues: known.issues.map((one) => issue(one)),
    comments: { issue: commented, texts: (comments[at(commented)] ?? []).toSorted() },
    timeEntries: {
      issue: entered,
      date,
      minutes: (entries[at(entered)] ?? [])
        .filter((entry) => entry.date === date)
        .map((entry) => entry.minutes)
        .toSorted((a, b) => a - b),
    },
    workDay: { ...known.workDay, periods: await workDay(known.workDay) },
  };
}

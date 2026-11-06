import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

type TimeEntry = Schema<"TimeEntryItem">;

/** One row of *Time on issues*: an issue and what was booked on it, day by day. */
export interface IssueRow {
  /** The internal ID of the issue: what an entry is booked with. */
  readonly issue: string;
  /** The readable ID (WEB-142) while the person sees the issue. */
  readonly readableId: string | undefined;
  /** The title of the issue ('' while the person's role hides it), as the entries say it. */
  readonly title: string;
  /** The value of *Activity* and of *Cost center* of the entries of the row, as the entries say them. */
  readonly activity: string;
  readonly costCenter: string;
  /** The entries of each day, by its date. */
  readonly entries: Readonly<Record<string, readonly TimeEntry[]>>;
}

/** The starter fields of time entries by their mark, as the timer's answer gives their IDs. */
export interface Starters {
  readonly activity: string | undefined;
  readonly costCenter: string | undefined;
}

export function startersOf(marks: Readonly<Record<string, string>> | undefined): Starters {
  return { activity: marks?.["activity"], costCenter: marks?.["cost-center"] };
}

/** The names of the choices the entries hold in a field, each once, as the server says them. */
function distinct(entries: readonly TimeEntry[], field: string | undefined): string {
  const names = entries.flatMap((entry) =>
    field === undefined ? [] : (entry.own_field_names[field] ?? []),
  );
  return [...new Set(names)].join(t("time.sheet.list-separator"));
}

/** The minutes of the entries of a cell. */
export function minutesOf(entries: readonly TimeEntry[]): number {
  return entries.reduce((sum, entry) => sum + entry.minutes, 0);
}

/** The entries grouped by what `key` says of each, in the order they came. */
function groupedBy(
  entries: readonly TimeEntry[],
  key: (entry: TimeEntry) => string,
): Readonly<Record<string, readonly TimeEntry[]>> {
  const groups: Record<string, readonly TimeEntry[]> = {};
  for (const entry of entries) {
    groups[key(entry)] = [...(groups[key(entry)] ?? []), entry];
  }
  return groups;
}

function nameOf(row: Pick<IssueRow, "issue" | "readableId">): string {
  return row.readableId ?? row.issue;
}

/**
 * The rows of a week: one for each issue with entries in it, by the readable ID, so the order says
 * nothing about anybody's time (ADR-0035, decision 9).
 */
export function issueRows(entries: readonly TimeEntry[], starters: Starters): readonly IssueRow[] {
  const rows = Object.values(groupedBy(entries, (entry) => entry.issue)).map((own): IssueRow => ({
    issue: own[0]?.issue ?? "",
    readableId: own[0]?.readable_id ?? undefined,
    title: own[0]?.issue_title ?? "",
    activity: distinct(own, starters.activity),
    costCenter: distinct(own, starters.costCenter),
    entries: groupedBy(own, (entry) => entry.date),
  }));
  return rows.toSorted((a, b) => nameOf(a).localeCompare(nameOf(b), undefined, { numeric: true }));
}

/** The minutes booked in a row over the days shown. */
export function rowTotal(row: IssueRow, days: readonly string[]): number {
  return days.reduce((sum, day) => sum + minutesOf(row.entries[day] ?? []), 0);
}

/**
 * The way into the details of a row: only a row of an issue the person can see has one (the ID of
 * a hidden issue is not told), the same for the grid and for a phone.
 */
export function detailsOpener(
  row: Pick<IssueRow, "issue" | "readableId">,
  onDetails: (issue: string) => void,
): (() => void) | undefined {
  return row.readableId === undefined
    ? undefined
    : () => {
        onDetails(row.issue);
      };
}

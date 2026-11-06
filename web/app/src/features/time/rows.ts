import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

type TimeEntry = Schema<"TimeEntryItem">;
type Choice = Schema<"FieldChoiceBody">;

/** The choices of a field that have these IDs, in the order the field lists them (by number). */
export type ChoicesHeld = (field: string, ids: readonly string[]) => readonly Choice[];

/** One row of *Time on issues*: an issue and what was booked on it, day by day. */
export interface IssueRow {
  /** The internal ID of the issue: what an entry is booked with. */
  readonly issue: string;
  /** The readable ID (WEB-142) while the person sees the issue. */
  readonly readableId: string | undefined;
  /** The title of the issue ('' while the person's role hides it), as the entries say it. */
  readonly title: string;
  /** The choices of *Activity* and of *Cost center* that the entries of the row hold, each once. */
  readonly activity: readonly Choice[];
  readonly costCenter: readonly Choice[];
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

/** The IDs a value of a *Pick one* or *Pick several* field holds. */
function idsOf(value: unknown): readonly string[] {
  if (typeof value === "string") {
    return [value];
  }
  return Array.isArray(value)
    ? value.filter((each): each is string => typeof each === "string")
    : [];
}

/**
 * The choices the entries hold in a field, each once, found in the field the browser has read:
 * the entries carry the IDs, so no name travels with each entry (M6-F95). They stand in the order the field lists them.
 */
function distinct(
  entries: readonly TimeEntry[],
  field: string | undefined,
  held: ChoicesHeld,
): readonly Choice[] {
  if (field === undefined) {
    return [];
  }
  return held(
    field,
    entries.flatMap((entry) => idsOf(entry.own_fields[field])),
  );
}

/** The texts of choices (`shown_as`) as one text, for a title that shows the whole when a cell cuts it short. */
export function choicesTitle(choices: readonly Choice[]): string {
  return choices.map((choice) => choice.shown_as).join(t("time.sheet.list-separator"));
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
export function issueRows(
  entries: readonly TimeEntry[],
  starters: Starters,
  held: ChoicesHeld,
): readonly IssueRow[] {
  const rows = Object.values(groupedBy(entries, (entry) => entry.issue)).map((own): IssueRow => ({
    issue: own[0]?.issue ?? "",
    readableId: own[0]?.readable_id ?? undefined,
    title: own[0]?.issue_title ?? "",
    activity: distinct(own, starters.activity, held),
    costCenter: distinct(own, starters.costCenter, held),
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

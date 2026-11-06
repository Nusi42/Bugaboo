import type { Wording } from "@bugaboo/wasm";

import { EntryDetailsWindow } from "./EntryDetailsWindow.tsx";
import type { IssueRow } from "./rows.ts";
import { useEntryDefaults } from "./use-entry-defaults.ts";
import { useEntryIssue } from "./use-entry-issue.ts";

export interface EntryDetailsProps {
  /** The row whose entries are shown. */
  readonly row: IssueRow;
  /** The dates of the week, in order. */
  readonly days: readonly string[];
  readonly wording: Wording;
  readonly onClose: () => void;
}

/**
 * The details of a row of the timesheet: the window with its entries, once the issue and what a
 * time entry on it starts with are read. A row of an issue the person cannot see has none.
 */
export function EntryDetails({ row, days, wording, onClose }: EntryDetailsProps) {
  const issue = useEntryIssue(row.readableId);
  const defaults = useEntryDefaults(row.readableId ?? "", row.readableId !== undefined).data;
  if (issue === undefined || defaults === undefined) {
    return null;
  }
  return (
    <EntryDetailsWindow
      issue={issue}
      entries={days.flatMap((date) => row.entries[date] ?? [])}
      defaults={defaults}
      wording={wording}
      onClose={onClose}
    />
  );
}

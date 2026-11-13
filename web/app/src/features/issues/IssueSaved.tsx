import { Alert, alertTone, SaveStatus, partOf, part } from "@bugaboo/ui";

import { saveLabels } from "../../shared/save-labels.ts";
import { ConflictNote } from "./ConflictNote.tsx";
import type { IssueChanges } from "./use-issue-changes.ts";

/**
 * *Saving…*, *Saved · 10:42* or *Not saved yet* for a change that has no row in the Details (the
 * title, the status). It stands in the line of the status button, beside it, so the page under it
 * does not move when it comes and goes (UX §6.4). A change in a row says it there.
 */
export function IssueSaveState({ changes }: { readonly changes: IssueChanges }) {
  return changes.lastRow === null ? (
    <SaveStatus state={changes.saveState} labels={saveLabels(changes.savedAt)} />
  ) : null;
}

/** What went wrong if the server refused a change, and a change that somebody else made first. */
export function IssueSaved({ changes }: { readonly changes: IssueChanges }) {
  return (
    <div {...partOf(part.issueNote)}>
      {changes.problem === null ? null : <Alert tone={alertTone.danger}>{changes.problem}</Alert>}
      {changes.conflict === null ? null : <ConflictNote conflict={changes.conflict} />}
    </div>
  );
}

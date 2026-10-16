import { Alert, alertTone, SaveStatus, partOf, part } from "@bugaboo/ui";

import { saveLabels } from "../../shared/save-labels.ts";
import { ConflictNote } from "./ConflictNote.tsx";
import type { IssueChanges } from "./use-issue-changes.ts";

/**
 * *Saving…*, *Saved · 10:42* or *Not saved yet* for a change that has no row in the Details (the
 * title, the status), and what went wrong if the server refused, and a change that somebody else made first. A change in a row says it there.
 */
export function IssueSaved({ changes }: { readonly changes: IssueChanges }) {
  return (
    <div {...partOf(part.issueNote)}>
      {changes.lastRow === null ? (
        <SaveStatus state={changes.saveState} labels={saveLabels(changes.savedAt)} />
      ) : null}
      {changes.problem === null ? null : <Alert tone={alertTone.danger}>{changes.problem}</Alert>}
      {changes.conflict === null ? null : <ConflictNote conflict={changes.conflict} />}
    </div>
  );
}

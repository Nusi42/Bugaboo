import { Alert, alertTone, SaveStatus, partOf, part } from "@bugaboo/ui";

import type { IssueChanges } from "./use-issue-changes.ts";
import { saveLabels } from "../../shared/save-labels.ts";

/** *Saving…*, *Saved · 10:42* or *Not saved yet*, and what went wrong if the server refused. */
export function IssueSaved({ changes }: { readonly changes: IssueChanges }) {
  return (
    <div {...partOf(part.issueNote)}>
      <SaveStatus state={changes.saveState} labels={saveLabels(changes.savedAt)} />
      {changes.problem === null ? null : <Alert tone={alertTone.danger}>{changes.problem}</Alert>}
    </div>
  );
}

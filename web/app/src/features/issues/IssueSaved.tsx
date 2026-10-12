import { t } from "@bugaboo/messages";
import { Alert, alertTone, SaveStatus, partOf, part } from "@bugaboo/ui";

import { formatTime } from "../../shared/format.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

/** *Saving…*, *Saved · 10:42* or *Not saved yet*, and what went wrong if the server refused. */
export function IssueSaved({ changes }: { readonly changes: IssueChanges }) {
  return (
    <div {...partOf(part.issueNote)}>
      <SaveStatus
        state={changes.saveState}
        labels={{
          saving: t("save.saving"),
          saved:
            changes.savedAt === null
              ? t("save.saved")
              : t("issues.saved-at", { time: formatTime(changes.savedAt) }),
          "not-saved": t("save.not-saved"),
        }}
      />
      {changes.problem === null ? null : <Alert tone={alertTone.danger}>{changes.problem}</Alert>}
    </div>
  );
}

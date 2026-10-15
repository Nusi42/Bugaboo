import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { refreshIssues } from "../../shared/query-keys.ts";
import type { IssueView } from "./slots.ts";

/**
 * Puts an issue into the trash at once, with *Undo* for 10 seconds (UX §6: a reversible action asks
 * no question). *Undo* takes it out again. `onDeleted` is called when the server has it, so the
 * page can go back to the list.
 */
export function deleteIssue(issue: IssueView, onDeleted: () => void): void {
  const key = known(issue.key) ?? known(issue.id);
  if (key === undefined) {
    return;
  }
  void api.client
    .DELETE("/api/v1/issues/{issue}", { params: { path: { issue: key } } })
    .then(() => {
      refreshIssues();
      onDeleted();
      undoStore.offer({
        message: t("issues.deleted", { key }),
        undo: () => {
          void api.client
            .POST("/api/v1/issues/{issue}/restore", { params: { path: { issue: key } } })
            .then(refreshIssues, refreshIssues);
        },
      });
    }, refreshIssues);
}

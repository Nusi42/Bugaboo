import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { known } from "../../shared/present.ts";
import { refreshIssues } from "../../shared/query-keys.ts";
import type { IssueView } from "./slots.ts";

/** What the caller does when the server has the issue in the trash, or refuses to. */
export interface DeleteHandlers {
  /** The server has it: the page can go back to the list. */
  readonly onDeleted: () => void;
  /** The server refused (somebody deleted it first, the right was taken away): the page says why. */
  readonly onRefused: (error: unknown) => void;
}

/**
 * Puts an issue into the trash at once, with *Undo* for 10 seconds (UX §6: a reversible action asks
 * no question). *Undo* takes it out again; when that is refused the person is told in a notice,
 * because the page they were on is gone. A refused delete stays on the page, which says why.
 */
export function deleteIssue(issue: IssueView, handlers: DeleteHandlers): void {
  const key = known(issue.key) ?? known(issue.id);
  if (key === undefined) {
    return;
  }
  void api.client.DELETE("/api/v1/issues/{issue}", { params: { path: { issue: key } } }).then(
    () => {
      refreshIssues();
      handlers.onDeleted();
      undoStore.offer({
        message: t("issues.deleted", { key }),
        undo: () => {
          void api.client
            .POST("/api/v1/issues/{issue}/restore", { params: { path: { issue: key } } })
            .then(refreshIssues, (error: unknown) => {
              refreshIssues();
              undoStore.offer({ message: messageOf(error) });
            });
        },
      });
    },
    // The page is not read again here: it would be replaced (the issue may be gone) and take the
    // reason with it. The next visit reads what is true.
    handlers.onRefused,
  );
}

import { t } from "@bugaboo/messages";
import type { ChangeStatus } from "@bugaboo/ui";

import { undoStore } from "../../shared/optimistic-undo.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import type { RowField } from "./issue-values.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

/**
 * What the row of a field in the Details says about the last change (UX §4.4, R7): *Saving…*,
 * *Saved* and *Undo*, only in the row where the change was made. A row that did not change says
 * nothing.
 */
export function rowStatus(changes: IssueChanges, field: RowField): ChangeStatus | undefined {
  const last = changes.lastRow;
  if (last?.field !== field) {
    return undefined;
  }
  const offer = last.undo;
  return {
    state: changes.saveState,
    labels: saveLabels(changes.savedAt),
    ...(offer === undefined
      ? {}
      : {
          undo: {
            label: t("common.undo"),
            onUndo: () => {
              undoStore.take(offer.id);
            },
          },
        }),
  };
}

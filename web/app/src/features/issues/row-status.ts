import { t } from "@bugaboo/messages";
import type { ChangeStatus, SaveState } from "@bugaboo/ui";

import { undoStore, type UndoOffer } from "../../shared/optimistic-undo.ts";
import { saveLabels } from "../../shared/save-labels.ts";
import type { RowField } from "./issue-values.ts";
import type { IssueChanges } from "./use-issue-changes.ts";

/** What the row of the last change of the Details needs to say how it stands. */
export interface RowChange<Key> {
  /** The row that changed last, with the offer to take it back while there is one. */
  readonly lastRow: { readonly field: Key; readonly undo: UndoOffer | undefined } | null;
  readonly saveState: SaveState;
  /** When the last change was saved, as a moment in time. */
  readonly savedAt: string | null;
}

/**
 * What the row of a field in the Details says about the last change (UX §4.4, R7): *Saving…*,
 * *Saved* and *Undo*, only in the row where the change was made. A row that did not change says
 * nothing. One helper for every row of the Details, whatever kind of key names it: *Undo* is the
 * offer of the store of undo, so the 10 seconds, the shortcut and the focus come with it.
 */
export function rowStatusOf<Key>(changes: RowChange<Key>, field: Key): ChangeStatus | undefined {
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

/** The status of a field of the issue itself (*Assigned to*, *Priority*, an own field). */
export function rowStatus(changes: IssueChanges, field: RowField): ChangeStatus | undefined {
  return rowStatusOf(changes, field);
}

import { api } from "../../shared/api.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshLabels } from "../labels/index.ts";
import { bulkKind, refreshIssues, type BulkAction, type BulkResult } from "./bulk.ts";

/** The IDs of the issues a change really changed: the ones that were not skipped. */
export function changedIssues(result: BulkResult): string[] {
  return result.outcomes.flatMap((outcome) =>
    outcome.issue !== undefined &&
    outcome.issue !== null &&
    (outcome.skipped === undefined || outcome.skipped === null)
      ? [outcome.issue.id]
      : [],
  );
}

/** Takes the change back for one issue, for the two things that can be taken back at once. */
async function takeBack(action: BulkAction, issue: string): Promise<void> {
  if (action.kind === bulkKind.label) {
    await api.client.DELETE("/api/v1/issues/{issue}/labels/{label}", {
      params: { path: { issue, label: action.label } },
    });
  } else if (action.kind === bulkKind.delete) {
    await api.client.POST("/api/v1/issues/{issue}/restore", { params: { path: { issue } } });
  }
}

/** Whether a change can be taken back by *Undo*: adding a label and deleting can. */
export function canUndo(action: BulkAction): boolean {
  return action.kind === bulkKind.label || action.kind === bulkKind.delete;
}

/**
 * Offers *Undo* for the ten seconds every undo is on offer for (UX §6): adding a label takes the
 * label off again, and deleting takes the issues out of the trash.
 */
export function offerUndo(action: BulkAction, result: BulkResult, message: string): void {
  const issues = changedIssues(result);
  if (!canUndo(action) || issues.length === 0) {
    return;
  }
  undoStore.offer({
    message,
    undo: () => {
      void Promise.all(issues.map((issue) => takeBack(action, issue))).then(() => {
        refreshIssues();
        refreshLabels();
      });
    },
  });
}

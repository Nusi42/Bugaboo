import { api } from "../../shared/api.ts";
import { undoStore } from "../../shared/optimistic-undo.ts";
import { refreshLabels, takeLabelOff } from "../labels/index.ts";
import { manyChangeKind, type ManyChangeAction, type ManyChangeResult } from "./many-changes.ts";
import { refreshIssues } from "../../shared/query-keys.ts";

/** The IDs of the issues a change really changed: the ones that were not skipped. */
export function changedIssues(result: ManyChangeResult): string[] {
  return result.outcomes.flatMap((outcome) =>
    outcome.issue !== undefined &&
    outcome.issue !== null &&
    (outcome.skipped === undefined || outcome.skipped === null)
      ? [outcome.issue.id]
      : [],
  );
}

/** Takes the change back for one issue, for the two things that can be taken back at once. */
async function takeBack(action: ManyChangeAction, issue: string): Promise<void> {
  if (action.kind === manyChangeKind.label) {
    await takeLabelOff(issue, action.label);
  } else if (action.kind === manyChangeKind.delete) {
    await api.client.POST("/api/v1/issues/{issue}/restore", { params: { path: { issue } } });
  }
}

/** Whether a change can be taken back by *Undo*: adding a label and deleting can. */
export function canUndo(action: ManyChangeAction): boolean {
  return action.kind === manyChangeKind.label || action.kind === manyChangeKind.delete;
}

/**
 * Offers *Undo* for the ten seconds every undo is on offer for (UX §6): adding a label takes the
 * label off again, and deleting takes the issues out of the trash.
 */
export function offerUndo(
  action: ManyChangeAction,
  result: ManyChangeResult,
  message: string,
): void {
  const issues = changedIssues(result);
  if (!canUndo(action) || issues.length === 0) {
    return;
  }
  undoStore.offer({
    message,
    undo: () => {
      // Whatever each one answers, the screens read again: they show what is true now, also when
      // one of the issues could not be taken back.
      void Promise.allSettled(issues.map((issue) => takeBack(action, issue))).then(() => {
        refreshIssues();
        refreshLabels();
      });
    },
  });
}

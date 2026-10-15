import { t } from "@bugaboo/messages";

import { refreshLabels } from "../labels/index.ts";
import { type BulkApplied } from "./bulk.ts";
import { changedIssues, offerUndo } from "./undo.ts";
import { refreshIssues } from "../../shared/query-keys.ts";

/**
 * What follows applying a change: the lists are read again, the ticks go away when it worked, and
 * what can be taken back is offered as *Undo*.
 */
export function afterApply(done: BulkApplied, clearTicks: () => void): void {
  refreshIssues();
  refreshLabels();
  if (done.failed === null) {
    clearTicks();
  }
  if (done.result !== null) {
    const changed = changedIssues(done.result).length;
    offerUndo(
      done.action,
      done.result,
      t("bulk.result-done", { changed, skipped: done.result.outcomes.length - changed }),
    );
  }
}

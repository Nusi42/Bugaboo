import { doNothing } from "@bugaboo/ui";

import type { RowDialogProps } from "../issues/index.ts";
import { afterApply } from "./after-apply.ts";
import { manyChangeKind } from "./many-changes.ts";
import { ManyChangeDialog } from "./ManyChangeDialog.tsx";

/**
 * *Change status…* of a row of the issue list (UX §3.3): the window that changes many issues at once, for this one
 * issue, so its status (or its priority) changes as a change of ticked issues does: shown first, then applied, with
 * *Undo*.
 */
export function RowStatusChange({ issue, onClose }: RowDialogProps) {
  return (
    <ManyChangeDialog
      kind={manyChangeKind.change}
      project={issue.project}
      issues={[issue.id]}
      isOpen
      onOpenChange={(next) => {
        if (!next) {
          onClose();
        }
      }}
      onApplied={(done) => {
        afterApply(done, doNothing);
        onClose();
      }}
    />
  );
}

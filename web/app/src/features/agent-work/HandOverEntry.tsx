import { useState } from "react";

import type { MoreDialogProps } from "../issues/index.ts";
import { HandOverDialog } from "./HandOverDialog.tsx";

/** The window of *Hand over to an agent* from the *More* of an issue: it opens at once, closes with *Cancel*. */
export function HandOverEntry({ issue, onClose, onProblem }: MoreDialogProps) {
  const [open, setOpen] = useState(true);
  const key = issue.readable_id ?? "";
  return (
    <HandOverDialog
      issues={[key]}
      issue={key}
      onFailed={onProblem}
      isOpen={open}
      onOpenChange={(next) => {
        setOpen(next);
        if (!next) {
          onClose();
        }
      }}
    />
  );
}

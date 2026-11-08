import { useState } from "react";

import { listText } from "../../shared/list-text.ts";
import { personName } from "../../shared/person.ts";
import type { MoreDialogProps } from "../issues/index.ts";
import { HandOverDialog } from "./HandOverDialog.tsx";

/** The window of *Hand over to an agent* from the *More* of an issue: it opens at once, closes with *Cancel*. */
export function HandOverEntry({ issue, onClose }: MoreDialogProps) {
  const [open, setOpen] = useState(true);
  const key = issue.readable_id ?? "";
  const people = (issue.assigned_to ?? []).map((person) => personName(person));
  return (
    <HandOverDialog
      issues={[key]}
      issue={key}
      assigned={people.length === 0 ? undefined : listText(people)}
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

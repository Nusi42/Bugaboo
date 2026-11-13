import { useState, type ReactNode } from "react";

import { RowDialogContext, type OpenRowDialog } from "./row-actions.ts";

/** Holds the window of an action of a row for the list around it, and draws it while it is open. */
export function RowDialogs({ children }: { readonly children: ReactNode }) {
  const [open, setOpen] = useState<OpenRowDialog | null>(null);
  return (
    <RowDialogContext value={setOpen}>
      {children}
      {open === null ? null : (
        <open.dialog
          issue={open.issue}
          onClose={() => {
            setOpen(null);
          }}
        />
      )}
    </RowDialogContext>
  );
}

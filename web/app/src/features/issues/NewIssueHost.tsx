import { lazy, Suspense } from "react";

import { useOverlayOpen } from "../shell/index.ts";

/** The dialog is loaded when it is first opened, so the first page stays small (coding-standards section 6). */
const NewIssueDialog = lazy(async () => ({
  default: (await import("./NewIssueDialog.tsx")).NewIssueDialog,
}));

/** Draws the new-issue dialog while it is open; the code of the dialog arrives with the first opening. */
export function NewIssueHost() {
  return useOverlayOpen("new-issue") ? (
    <Suspense fallback={null}>
      <NewIssueDialog />
    </Suspense>
  ) : null;
}

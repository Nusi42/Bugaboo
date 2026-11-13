import { t } from "@bugaboo/messages";
import { lazy, Suspense, useEffect } from "react";

import { idlePreloadMs, startPreload } from "../../shared/preload.ts";
import { WaitingWindow } from "../../shared/WaitingWindow.tsx";
import { windowForms } from "../../shared/window-forms.ts";
import { closeOverlay, useOverlayOpen } from "../shell/index.ts";

/** Fetches the code of the dialog; the browser keeps the module, so it is fetched once. */
function loadDialog(): Promise<typeof import("./NewIssueDialog.tsx")> {
  return import("./NewIssueDialog.tsx");
}

/** The dialog is loaded when it is first opened, so the first page stays small (coding-standards section 6). */
const NewIssueDialog = lazy(async () => ({ default: (await loadDialog()).NewIssueDialog }));

/**
 * Draws the new-issue dialog while it is open. Its code is fetched soon after the page is up, and
 * until it is there the window waits as it does for its fields: dimmed page at once, grey shapes
 * after 0.3 s, Esc closes it.
 */
export function NewIssueHost() {
  useEffect(() => startPreload(loadDialog, idlePreloadMs), []);
  return useOverlayOpen("new-issue") ? (
    <Suspense
      fallback={
        <WaitingWindow
          form={windowForms.newIssue}
          title={t("issues.new-title")}
          onClose={closeOverlay}
        />
      }
    >
      <NewIssueDialog />
    </Suspense>
  ) : null;
}

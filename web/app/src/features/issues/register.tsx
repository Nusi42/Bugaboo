import { t } from "@bugaboo/messages";

import { addOverlayHost, addQuickAction, addQuickSearchSource } from "../shell/index.ts";
import { openNewIssue } from "./new-issue-state.ts";
import { NewIssueHost } from "./NewIssueHost.tsx";
import { issuesSource } from "./quick-search.ts";

let registered = false;

/**
 * Puts the new-issue dialog where the shell has room for it (the extension point of the shell's
 * layers), and issues and *Create issue* into quick search. Called once when the app starts.
 */
export function registerIssues(): void {
  if (registered) {
    return;
  }
  registered = true;
  addOverlayHost({ id: "new-issue", render: () => <NewIssueHost /> });
  addQuickSearchSource(issuesSource);
  addQuickAction({
    id: "create-issue",
    title: t("search.create-issue"),
    run: () => {
      openNewIssue();
    },
  });
}

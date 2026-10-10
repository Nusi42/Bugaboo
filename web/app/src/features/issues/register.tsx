import { addOverlayHost } from "../shell/index.ts";
import { NewIssueHost } from "./NewIssueHost.tsx";

let registered = false;

/**
 * Puts the new-issue dialog where the shell has room for it (the extension point of the shell's
 * layers). Called once when the app starts.
 */
export function registerIssues(): void {
  if (registered) {
    return;
  }
  registered = true;
  addOverlayHost({ id: "new-issue", render: () => <NewIssueHost /> });
}

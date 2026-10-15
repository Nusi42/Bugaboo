import { useMemo } from "react";

import { useVersionWatch } from "../../shared/editing/index.ts";
import { versionOf } from "../../shared/version.ts";
import { readAgain } from "./issue-requests.ts";
import type { IssueView } from "./slots.ts";

/**
 * The check every 30 seconds and on return to the tab (ADR-0019) that reads the issue again when
 * somebody else changed it. Gives the version the issue shows now.
 */
export function useIssueWatch(issue: IssueView, address: string): number {
  const version = versionOf(issue);
  const watched = useMemo(
    () => [{ record: { kind: "issue", id: issue.id ?? address }, version }],
    [issue.id, address, version],
  );
  useVersionWatch(watched, () => {
    readAgain(address);
  });
  return version;
}

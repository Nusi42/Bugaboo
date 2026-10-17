import { useSearch } from "@tanstack/react-router";

import type { IssueTarget } from "./paths.ts";

/**
 * Where on the issue page *Open* of a notification sent the person: a comment, or the change that
 * happened at a moment. Nothing when the address says neither.
 */
export function useIssueTarget(): IssueTarget | undefined {
  const { comment, change } = useSearch({ strict: false });
  if (typeof comment === "string") {
    return { kind: "comment", comment };
  }
  return typeof change === "string" ? { kind: "change", at: change } : undefined;
}

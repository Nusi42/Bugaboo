import type { Schema } from "@bugaboo/api-client";

/** How much an agent may do, as the API writes it. */
export type Limits = Schema<"AgentLimitsBody">;

/** The three limits as the person typed them. */
export interface LimitsDraft {
  readonly changes: string;
  readonly issues: string;
  readonly attempts: string;
}

/** The draft of limits that stand now. */
export function draftOf(limits: Limits): LimitsDraft {
  return {
    changes: String(limits.changes_a_minute),
    issues: String(limits.issues_at_once),
    attempts: String(limits.attempts_on_one_issue),
  };
}

/** A whole number from 1 to `most`, or none for any other text. */
function numberWithin(text: string, most: number): number | undefined {
  const trimmed = text.trim();
  if (!/^\d+$/u.test(trimmed)) {
    return undefined;
  }
  const value = Number(trimmed);
  return value >= 1 && value <= most ? value : undefined;
}

/** The limits the draft says, or none while a number is no whole number from 1 to the most the organization allows. */
export function limitsOf(draft: LimitsDraft, most: Limits): Limits | undefined {
  const changes = numberWithin(draft.changes, most.changes_a_minute);
  const issues = numberWithin(draft.issues, most.issues_at_once);
  const attempts = numberWithin(draft.attempts, most.attempts_on_one_issue);
  return changes === undefined || issues === undefined || attempts === undefined
    ? undefined
    : { changes_a_minute: changes, issues_at_once: issues, attempts_on_one_issue: attempts };
}

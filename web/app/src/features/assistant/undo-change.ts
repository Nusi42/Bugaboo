import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { versionOf } from "../../shared/version.ts";
import type { CardChange } from "./ProposalCard.tsx";

/** What the way back puts back: the fields of an issue that `Change an issue` takes whole. */
interface Putback {
  readonly title?: string;
  readonly priority?: Schema<"Priority">;
  readonly assigned_to?: readonly string[];
}

/** The body of the change that puts the fields back. */
interface BackBody {
  version: number;
  title?: string;
  priority?: Schema<"Priority">;
  assigned_to?: string[];
}

const PRIORITIES: readonly Schema<"Priority">[] = ["low", "normal", "high", "critical"];

/** Whether a value is a list of texts. */
function isTexts(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((one) => typeof one === "string");
}

/** The value a field had, as the body of a change takes it; none when the field cannot be put back. */
function backTo(change: CardChange): Putback | undefined {
  const before = change.before;
  if (change.field === "title" && typeof before === "string") {
    return { title: before };
  }
  const priority = PRIORITIES.find((each) => each === before);
  if (change.field === "priority" && priority !== undefined) {
    return { priority };
  }
  if (change.field === "assigned-to" && isTexts(before)) {
    return { assigned_to: before };
  }
  return undefined;
}

/** Whether what a proposal changed can be taken back: fields of an issue that have a value before. */
export function canTakeBack(tool: string, changes: readonly CardChange[]): boolean {
  return (
    tool === "update_issue" &&
    changes.length > 0 &&
    changes.every((change) => backTo(change) !== undefined)
  );
}

/**
 * Puts back what *Apply* saved, as the person: the fields get the value they had, on the version
 * the issue has now. When somebody changed the issue meanwhile the server refuses, as for any
 * change, and nothing is overwritten.
 */
export async function takeBack(issue: string, changes: readonly CardChange[]): Promise<void> {
  const path = { params: { path: { issue } } };
  const { data } = await api.client.GET("/api/v1/issues/{issue}", path);
  const body: BackBody = { version: versionOf(data?.issue ?? {}) };
  for (const change of changes) {
    const back = backTo(change);
    if (back?.title !== undefined) {
      body.title = back.title;
    }
    if (back?.priority !== undefined) {
      body.priority = back.priority;
    }
    if (back?.assigned_to !== undefined) {
      body.assigned_to = [...back.assigned_to];
    }
  }
  await api.client.PATCH("/api/v1/issues/{issue}", { ...path, body });
  refreshIssues();
  refreshHistory();
}

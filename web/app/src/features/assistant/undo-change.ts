import { firstVersion, type Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { versionOf } from "../../shared/version.ts";
import type { AppliedProposal } from "./assistant.ts";
import type { CardChange } from "./ProposalCard.tsx";

/** What the way back puts back: the fields of an issue that `Change an issue` takes whole. */
interface Putback {
  readonly title?: string;
  readonly priority?: Schema<"Priority">;
  readonly assigned_to?: readonly string[];
  readonly status?: string;
}

/** The body of the change that puts the fields back. */
interface BackBody {
  version: number;
  title?: string;
  priority?: Schema<"Priority">;
  assigned_to?: string[];
  status?: string;
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
  if (change.field === "status" && typeof before === "string") {
    return { status: before };
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

/** Puts the fields of an issue back, as the person, on the version the issue has now. */
async function putFieldsBack(issue: string, changes: readonly CardChange[]): Promise<void> {
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
    if (back?.status !== undefined) {
      body.status = back.status;
    }
    if (back?.assigned_to !== undefined) {
      body.assigned_to = [...back.assigned_to];
    }
  }
  await api.client.PATCH("/api/v1/issues/{issue}", { ...path, body });
}

/** Takes away what an applied change made: a comment, a time entry, an issue or the running timer. */
async function takeAway(kind: string, id: string): Promise<void> {
  if (kind === "comment") {
    await api.client.DELETE("/api/v1/comments/{comment}", { params: { path: { comment: id } } });
  } else if (kind === "time-entry") {
    // A time entry that was just made is at its first version.
    await api.client.DELETE("/api/v1/me/time-entries/{entry}", {
      params: { path: { entry: id }, query: { version: firstVersion } },
    });
  } else if (kind === "issue") {
    await api.client.DELETE("/api/v1/issues/{issue}", { params: { path: { issue: id } } });
  } else {
    const { data } = await api.client.GET("/api/v1/me/timer");
    await api.client.POST("/api/v1/me/timer/throw-away", {
      body: { version: versionOf(data ?? {}) },
    });
  }
}

/** Whether what a proposal did can be taken back: something it made, or fields that had a value before. */
export function canTakeBack(
  issue: string | undefined,
  applied: Pick<AppliedProposal, "undo">,
  changes: readonly CardChange[],
): boolean {
  if (applied.undo !== undefined && applied.undo !== null) {
    return true;
  }
  return (
    issue !== undefined &&
    changes.length > 0 &&
    changes.every((change) => backTo(change) !== undefined)
  );
}

/**
 * Puts back what *Apply* saved, as the person. What it made is taken away (a comment, a time entry
 * and an issue go to the trash as for any delete); fields get the value they had, on the version the
 * issue has now. When somebody changed the issue meanwhile the server refuses, as for any change, and
 * nothing is overwritten.
 */
export async function takeBack(
  issue: string | undefined,
  applied: Pick<AppliedProposal, "undo">,
  changes: readonly CardChange[],
): Promise<void> {
  const made = applied.undo;
  if (made !== undefined && made !== null) {
    await takeAway(made.kind, made.id);
  } else if (issue !== undefined) {
    await putFieldsBack(issue, changes);
  }
  refreshIssues();
  refreshHistory();
}

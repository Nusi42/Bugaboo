import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import { refreshHistory, refreshIssues } from "../../shared/query-keys.ts";
import { versionOf } from "../../shared/version.ts";
import { sendChange, type IssueValues } from "../issues/index.ts";
import type { AppliedProposal } from "./assistant.ts";
import type { CardChange } from "./ProposalCard.tsx";

/** What was made changed since, so it is not taken away. */
class ChangedSince extends Error {}

const PRIORITIES: readonly Schema<"Priority">[] = ["low", "normal", "high", "critical"];

/** Whether a value is a list of texts. */
function isTexts(value: unknown): value is readonly string[] {
  return Array.isArray(value) && value.every((one) => typeof one === "string");
}

/** The value a field had, as the one change of an issue takes it; none when the field cannot be put back. */
function valueBefore(change: CardChange): IssueValues | undefined {
  const before = change.before;
  if (change.field === "title" && typeof before === "string") {
    return { title: before };
  }
  if (change.field === "status" && typeof before === "string") {
    // The request names the status by its key only.
    return { status: { key: before, name: before, status_group: "not-started" } };
  }
  const priority = PRIORITIES.find((each) => each === before);
  if (change.field === "priority" && priority !== undefined) {
    return { priority };
  }
  if (change.field === "assigned-to" && isTexts(before)) {
    return { assignedTo: before.map((id) => ({ id, name: id })) };
  }
  return undefined;
}

/** Whether what a proposal did can be taken back: something it made, or fields of an issue it left at a known version. */
export function canTakeBack(
  issue: string | undefined,
  applied: Pick<AppliedProposal, "undo" | "version">,
  changes: readonly CardChange[],
): boolean {
  if (applied.undo !== undefined && applied.undo !== null) {
    return true;
  }
  return (
    issue !== undefined &&
    applied.version !== undefined &&
    applied.version !== null &&
    changes.length > 0 &&
    changes.every((change) => valueBefore(change) !== undefined)
  );
}

/** The issue is still at the version it was made at. */
async function unchanged(issue: string, version: number): Promise<void> {
  const { data } = await api.client.GET("/api/v1/issues/{issue}", {
    params: { path: { issue } },
  });
  if (versionOf(data?.issue ?? {}) !== version) {
    throw new ChangedSince();
  }
}

/** Takes away what an applied change made: a comment, a time entry, an issue or the running timer. */
async function takeAway(made: NonNullable<AppliedProposal["undo"]>): Promise<void> {
  if (made.kind === "comment") {
    await api.client.DELETE("/api/v1/comments/{comment}", {
      params: { path: { comment: made.id } },
    });
  } else if (made.kind === "time-entry") {
    await api.client.DELETE("/api/v1/me/time-entries/{entry}", {
      params: { path: { entry: made.id }, query: { version: made.version } },
    });
  } else if (made.kind === "issue") {
    await unchanged(made.id, made.version);
    await api.client.DELETE("/api/v1/issues/{issue}", { params: { path: { issue: made.id } } });
  } else {
    await api.client.POST("/api/v1/me/timer/throw-away", { body: { version: made.version } });
  }
}

/**
 * Puts back what *Apply* saved, as the person. What it made is taken away (a comment and an issue go
 * to the trash as for any delete, a booked time entry on the version it was booked at); fields get the
 * value they had, in the one change of an issue, on the version *Apply* left: a change somebody else
 * made since comes back as a conflict, and nothing of theirs is overwritten.
 */
export async function takeBack(
  issue: string | undefined,
  applied: Pick<AppliedProposal, "undo" | "version">,
  changes: readonly CardChange[],
): Promise<void> {
  const made = applied.undo;
  if (made !== undefined && made !== null) {
    await takeAway(made);
  } else if (issue !== undefined && applied.version !== undefined && applied.version !== null) {
    const values: IssueValues = {};
    for (const change of changes) {
      Object.assign(values, valueBefore(change));
    }
    await sendChange(issue, { values, version: applied.version });
  }
  refreshIssues();
  refreshHistory();
}

/** What to tell the person when *Undo* did not work: the server's sentence, or that it changed since. */
export function undoProblem(error: unknown): string {
  return error instanceof ChangedSince ? t("assistant.card.undo-changed") : messageOf(error);
}

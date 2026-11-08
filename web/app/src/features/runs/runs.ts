import type { Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { fieldNameOf } from "../../shared/field-names.ts";
import { calendarDayOf, formatDateTime, formatTime } from "../../shared/format.ts";
import { pageSize, usePaged } from "../../shared/paged.ts";
import { personName } from "../../shared/person.ts";
import { refreshIssues, refreshRoutes, reread } from "../../shared/query-keys.ts";
import { runKeys } from "../../shared/run-keys.ts";

export type Run = Schema<"RunBody">;
export type RunChange = Schema<"RunChangeBody">;
export type RunField = Schema<"RunFieldBody">;
export type FieldOutcome = Schema<"FieldOutcomeBody">;
export type UndoPreview = Schema<"UndoBody">;
export type UndoSkip = Schema<"UndoSkipBody">;

/** The run of an AI app, an agent or a change to many issues. */
export function useRun(run: string) {
  return api.query.useQuery("get", "/api/v1/runs/{run}", { params: { path: { run } } });
}

/** The changes of a run, a page at a time: *Show more* reads the next page. */
export function useRunChanges(run: string) {
  return usePaged({
    queryKey: runKeys.changes(run),
    read: (after) =>
      api.client.GET("/api/v1/runs/{run}/changes", {
        params: { path: { run }, query: { limit: pageSize.usual, ...after } },
      }),
  });
}

/** A run was undone: the run, its changes, what undoing it would do and the issues are read again. */
export function refreshRun(): void {
  reread({ queryKey: runKeys.all });
  refreshRoutes(["/api/v1/runs"]);
  refreshIssues();
}

/** What undoing the run would do now; asked for when the person wants to look. */
export function useUndoPreview(run: string, enabled: boolean) {
  return api.query.useQuery(
    "get",
    "/api/v1/runs/{run}/undo",
    { params: { path: { run } } },
    { enabled, gcTime: 0, staleTime: 0 },
  );
}

/** Undoes the run with the version of the preview the person looked at. */
export function useUndoRun() {
  return api.query.useMutation("post", "/api/v1/runs/{run}/undo");
}

/** The title of a run: what it is of, with the name of the app. */
export function runTitle(run: Run): string {
  if (run.kind === "attempt") {
    return t("runs.title-attempt");
  }
  if (run.kind === "many-changes") {
    return t("runs.title-many-changes");
  }
  const app = run.app ?? "";
  return app === "" ? t("runs.title-app-session") : t("runs.title-app-session-named", { app });
}

/** Where a run is, in the words of a run (not of an issue): *Finished*, *Undone by Ada, …*. */
export function runStateText(run: Run): string {
  if (run.undone !== undefined && run.undone !== null) {
    return t(run.undone.interrupted ? "runs.state-undo-stopped" : "runs.state-undone", {
      person: personName(run.undone.by),
      at: formatDateTime(run.undone.at),
    });
  }
  if (run.status === "queued" || run.status === "running") {
    return t("runs.state-running");
  }
  if (run.status === "failed") {
    return t("runs.state-failed");
  }
  return run.status === "stopped" ? t("runs.state-stopped") : t("runs.state-finished");
}

/** Who made the run and when: *Ada Brandt via Claude Code, Tue 6 Oct, 11:02–11:31*. */
export function runActorLine(run: Run): string {
  const from = formatDateTime(run.started_at);
  const ended = run.ended_at ?? undefined;
  const sameDay =
    ended !== undefined &&
    calendarDayOf(Date.parse(run.started_at)) === calendarDayOf(Date.parse(ended));
  const to = ended === undefined ? undefined : sameDay ? formatTime(ended) : formatDateTime(ended);
  // One time when the run began and ended in the same minute.
  const sameMinute =
    sameDay && ended !== undefined && formatTime(ended) === formatTime(run.started_at);
  const span = to === undefined || sameMinute ? from : t("runs.span", { from, to });
  const person = personName(run.person);
  return run.app === undefined || run.app === null || run.app === ""
    ? t("runs.actor-plain", { person, span })
    : t("runs.actor-via", { person, app: run.app, span });
}

/** When a change of the run was made: the time alone on the day the run began, the date and time on another day. */
export function changeWhenText(run: Run, at: string): string {
  return calendarDayOf(Date.parse(at)) === calendarDayOf(Date.parse(run.started_at))
    ? formatTime(at)
    : formatDateTime(at);
}

/** What became of a field once the run was undone, in words; none before. */
export function outcomeText(outcome: FieldOutcome | null | undefined): string | undefined {
  if (outcome === undefined || outcome === null) {
    return undefined;
  }
  if (outcome.kind === "went-back") {
    return t("runs.outcome-went-back");
  }
  if (outcome.kind === "not-yet") {
    return t("runs.outcome-not-yet");
  }
  if (outcome.reason === "changed-since") {
    return outcome.by === undefined || outcome.by === null
      ? t("runs.outcome-kept-changed")
      : t("runs.outcome-kept-by", { person: personName(outcome.by) });
  }
  if (outcome.reason === "gone") {
    return t("runs.outcome-kept-gone");
  }
  return outcome.reason === "not-allowed"
    ? t("runs.outcome-kept-not-allowed")
    : t("runs.outcome-kept-not-undoable");
}

/** Why a field stays when the run is undone, as a sentence of the question. */
export function skipReasonText(skip: UndoSkip): string {
  if (skip.kind === "changed-since") {
    if (skip.by_you) {
      return t("runs.skip-by-you");
    }
    return skip.by === undefined || skip.by === null
      ? t("runs.skip-changed")
      : t("runs.skip-by", { person: personName(skip.by) });
  }
  if (skip.kind === "gone") {
    return t("runs.skip-gone");
  }
  return skip.kind === "not-allowed" ? t("runs.skip-not-allowed") : t("runs.skip-not-undoable");
}

/** One skipped field of the question: the issue, the field by its name, and why. */
export function skipLineText(skip: UndoSkip): string {
  const issue = skip.issue?.readable_id ?? t("runs.unknown-issue");
  const reason = skipReasonText(skip);
  // A comment is named under its issue; it cannot be put back yet.
  return skip.kind === "comment"
    ? t("runs.skip-line-comment", { issue, reason })
    : t("runs.skip-line", {
        issue,
        field: fieldNameOf({ code: skip.field, name: skip.name }),
        reason,
      });
}

/** How many fields stay that the preview counts but does not list: the list holds only the first ones. */
export function unlistedSkips(preview: {
  readonly skipped: readonly unknown[];
  readonly skipped_count: number;
}): number {
  return preview.skipped_count - preview.skipped.length;
}

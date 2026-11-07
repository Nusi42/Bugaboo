import type { Schema } from "@bugaboo/api-client";
import { isMessageKey, t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";

export type Run = Schema<"RunBody">;
export type RunChanges = Schema<"RunChangesBody">;
export type RunChange = Schema<"RunChangeBody">;
export type UndoPreview = Schema<"UndoBody">;
export type UndoSkip = Schema<"UndoSkipBody">;

/** The run of an AI app, an agent or a change to many issues. */
export function useRun(run: string) {
  return api.query.useQuery("get", "/api/v1/runs/{run}", { params: { path: { run } } });
}

/** One page of the changes of a run, after the event the page before ended with. */
export function useRunChanges(run: string, after: string | undefined) {
  return api.query.useQuery("get", "/api/v1/runs/{run}/changes", {
    params: { path: { run }, query: after === undefined ? {} : { after } },
  });
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

/** Where a run is, in the words of the status of a change to many issues. */
export function runStatusText(run: Run): string {
  const key = `many-changes-status.${run.status}`;
  return isMessageKey(key) ? t(key) : run.status;
}

/** Why a field stays when the run is undone. */
export function skipReasonText(skip: UndoSkip): string {
  if (skip.kind === "changed-since") {
    return skip.by_you ? t("runs.skip-changed-since-by-you") : t("runs.skip-changed-since");
  }
  if (skip.kind === "gone") {
    return t("runs.skip-gone");
  }
  return skip.kind === "not-allowed" ? t("runs.skip-not-allowed") : t("runs.skip-not-undoable");
}

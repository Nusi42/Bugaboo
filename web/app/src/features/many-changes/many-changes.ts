import type { components, Schema } from "@bugaboo/api-client";
import { isMessageKey, t } from "@bugaboo/messages";

import { api } from "../../shared/api.ts";
import { messageOf } from "../../shared/errors.ts";
import type { WordingState } from "../../shared/use-wording.ts";

/** What is done to many issues at once: one of five things. */
export type ManyChangeAction = Schema<"ManyChangeActionBody">;
/** The same, as it is built to be sent. */
export type NewManyChangeAction = components["schemas"]["ManyChangeActionBody"];
/** The word that says which of the five. */
export type ManyChangeKind = ManyChangeAction["kind"];
export type ManyChangePreview = Schema<"ManyChangePreviewBody">;
export type ManyChangeRow = Schema<"ManyChangeRowBody">;
export type ManyChangeResult = Schema<"ManyChangeResultBody">;
export type ManyChangeRun = Schema<"ManyChangeRunBody">;

/** The five things, as the words the server knows them by. */
export const manyChangeKind = {
  change: "change",
  move: "move",
  label: "add_label",
  relation: "add_relation",
  delete: "delete",
} as const satisfies Record<string, ManyChangeKind>;

/** What a change to many issues would do to each issue, without doing it. */
export function usePreviewManyChanges() {
  return api.query.useMutation("post", "/api/v1/many-changes/preview");
}

/** What was sent when a change was applied: the thing that was done. */
type SentManyChanges = Readonly<{ body: Readonly<{ action: ManyChangeAction }> }>;

/** What came of applying a change: what the server answered, or why it did not work. */
export interface ManyChangeApplied {
  readonly action: ManyChangeAction;
  readonly result: ManyChangeResult | null;
  readonly failed: string | null;
}

/**
 * Changes the issues the preview showed, each on the version the preview was made from. What came
 * of it is reported to `onApplied` also when the dialog that asked is gone by then.
 */
export function useApplyManyChanges(onApplied: (applied: ManyChangeApplied) => void) {
  return api.query.useMutation("post", "/api/v1/many-changes", {
    onSuccess: (result: ManyChangeResult, sent: SentManyChanges) => {
      onApplied({ action: sent.body.action, result, failed: null });
    },
    onError: (error: unknown, sent: SentManyChanges) => {
      onApplied({ action: sent.body.action, result: null, failed: messageOf(error) });
    },
  });
}

/** Whether a change that goes on in the background is still going. */
export function isRunning(run: ManyChangeRun): boolean {
  return run.status === "queued" || run.status === "running";
}

/** How far a change in the background is; it is read every second until it is over. */
export function useManyChangeRun(run: string | undefined) {
  return api.query.useQuery(
    "get",
    "/api/v1/many-changes/runs/{run}",
    { params: { path: { run: run ?? "" } } },
    {
      enabled: run !== undefined,
      refetchInterval: (
        query: Readonly<{ state: Readonly<{ data: ManyChangeRun | undefined }> }>,
      ) => {
        const known = query.state.data;
        return known !== undefined && !isRunning(known) ? false : 1000;
      },
    },
  );
}

/** The issues a preview would change, with the version each was read at: what the change is sent for. */
export function targetsOf(preview: ManyChangePreview): { issue: string; version: number }[] {
  return preview.rows.flatMap((row) =>
    row.issue !== undefined &&
    row.issue !== null &&
    (row.skipped === undefined || row.skipped === null) &&
    row.version !== undefined &&
    row.version !== null
      ? [{ issue: row.issue.id, version: row.version }]
      : [],
  );
}

/** Why an issue is left alone, in words. */
export function skipText(reason: Schema<"SkipReason">): string {
  const key = `many-changes-skip.${reason}`;
  return isMessageKey(key) ? t(key) : reason;
}

/**
 * Whether *Apply* may be pressed: the dialog allows it, and the words of the preview are there. A
 * preview the person could not read (the words failed to load, or are still loading) must not let
 * them start the change blind.
 */
export function mayApply(canApply: boolean, words: WordingState): boolean {
  return canApply && words.state === "ready";
}

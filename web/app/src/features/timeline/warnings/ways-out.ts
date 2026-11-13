import { dataOf, type Schema } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";

import { api } from "../../../shared/api.ts";
import { messageOf } from "../../../shared/errors.ts";
import { undoStore } from "../../../shared/optimistic-undo.ts";
import type { TimelineOptions } from "../../../shared/list-view.ts";
import { namedPerson, type NamedPerson } from "../../../shared/person.ts";
import { sendChange } from "../../issues/index.ts";
import { idOf, refreshTimeline, type PlanWarning, type TimelineRow } from "../timeline-data.ts";

/**
 * The ways out of a plan warning that go to the server (UX §4.7, PLAN-13): the preview of *Move it
 * after*, the move itself as one run with *Undo*, and *Keep and accept*. The server judges each;
 * a refusal is said in its words.
 */

/** One issue of the preview of *Move it after*, with the dates it gets. */
export type MovePreviewIssue = Schema<"MoveAfterIssueBody">;

/** The preview of *Move it after*: every issue that would move, with its new days. Saves nothing. */
export async function previewMoveAfter(
  warning: PlanWarning,
  options: TimelineOptions,
): Promise<readonly MovePreviewIssue[]> {
  const preview = dataOf(
    await api.client.POST("/api/v1/timeline/move-after", {
      body: { issue: warning.issue, after: warning.move_after ?? "", timeline: options },
    }),
  );
  return preview.issues;
}

/** Takes a run back: the preview of its undo first, so a change made since is never written over. */
async function undoRun(run: string): Promise<void> {
  try {
    const preview = dataOf(
      await api.client.GET("/api/v1/runs/{run}/undo", { params: { path: { run } } }),
    );
    dataOf(
      await api.client.POST("/api/v1/runs/{run}/undo", {
        params: { path: { run } },
        body: { preview_version: preview.preview_version },
      }),
    );
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/** Moves the issues of a preview in one run, and offers *Undo* for the whole run. */
export async function moveAsPreviewed(issues: readonly MovePreviewIssue[]): Promise<void> {
  try {
    const moved = dataOf(
      await api.client.POST("/api/v1/issues/move-dates", {
        body: {
          // A date the reader may not read is not in the preview: it is left as it is.
          issues: issues.map((each) => ({
            issue: each.id,
            version: each.version,
            ...(each.start_date === null || each.start_date === undefined
              ? {}
              : { start_date: each.start_date }),
            ...(each.due_date === null || each.due_date === undefined
              ? {}
              : { due_date: each.due_date }),
          })),
        },
      }),
    );
    undoStore.offer({
      message: t("timeline.moved-after", { count: moved.issues.length }),
      undo: () => {
        void undoRun(moved.run);
      },
    });
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/** *Keep and accept*: the warning stays, faded, until the dates of its issue change. */
export async function keepAndAccept(
  warning: PlanWarning,
  issue: { readonly key: string; readonly version: number },
  options: TimelineOptions,
): Promise<void> {
  try {
    dataOf(
      await api.client.POST("/api/v1/timeline/accepted-warnings", {
        body: {
          issue: warning.issue,
          version: issue.version,
          kind: warning.kind,
          timeline: options,
        },
      }),
    );
    undoStore.offer({ message: t("timeline.accepted", { key: issue.key }) });
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/**
 * *Give it to someone else…*: the person picked in the usual picker becomes the person on the bar;
 * the others assigned after the first stay. Only people the reader sees by name can be named again,
 * so the way is offered only when every assigned person is shown by name (`mayGive`).
 */
export async function giveTo(
  row: TimelineRow,
  person: NamedPerson,
  version: number,
): Promise<void> {
  const others = (row.issue.assigned_to ?? [])
    .slice(1)
    .flatMap((each) => namedPerson(each) ?? [])
    .filter((each) => each.id !== person.id);
  try {
    await sendChange(idOf(row), { version, values: { assignedTo: [person, ...others] } });
    undoStore.offer({
      message: t("timeline.given", { key: row.issue.readable_id ?? "", name: person.name }),
    });
  } catch (error) {
    undoStore.offer({ message: messageOf(error) });
  }
  await refreshTimeline();
}

/** Whether the issue of a row can be given to someone else here: everyone assigned is named. */
export function mayGive(row: TimelineRow): boolean {
  return (row.issue.assigned_to ?? []).every((each) => namedPerson(each) !== undefined);
}

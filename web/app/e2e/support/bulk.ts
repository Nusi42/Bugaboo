import { dataOf, type components, type Schema } from "@bugaboo/api-client";
import { expect } from "@playwright/test";

import { type ApiSession } from "./api.ts";
import { type Changes, changesOf, changesSince, readIssue } from "./crowd-checks.ts";
import { type MadeIssue } from "./issues.ts";

// Changing many issues (ISS-09, M3 WP-08), as the API description has it: a preview that reads the
// issues, then the change, which takes back the versions the preview read. This file is the one
// place that knows the shape of both; the scenarios use nothing else of it.

/** The status an issue can have. */
type NewStatus = components["schemas"]["Status"];

/** What is done to every issue of the selection (a body that is sent is not read-only). */
type Action = components["schemas"]["BulkActionBody"];

/** The body that changes the issues, each on a version. */
type ApplyBody = components["schemas"]["ApplyBulkBody"];

/** What the preview said, one row for each issue asked for. */
export type BulkPreview = Schema<"BulkPreviewBody">;

/** What the change did: one outcome for each issue. */
export type BulkResult = Schema<"BulkResultBody">;

/** The one thing done to every issue: a new status. */
function statusAction(status: NewStatus): Action {
  return { kind: "change", change: { status } };
}

/** Asks what changing the status of the issues would do; nothing is changed. */
export async function previewStatus(
  session: ApiSession,
  keys: readonly string[],
  status: NewStatus,
): Promise<BulkPreview> {
  return dataOf(
    await session.client.POST("/api/v1/bulk/preview", {
      body: { issues: [...keys], action: statusAction(status) },
    }),
  );
}

/**
 * The body that changes the status of the issues a preview found changeable, each on the version
 * the preview read; a row the preview left alone is not sent.
 */
export function bulkBody(preview: BulkPreview, status: NewStatus): ApplyBody {
  return {
    issues: preview.rows.flatMap((row) =>
      row.issue === undefined ||
      row.issue === null ||
      row.version === undefined ||
      row.version === null
        ? []
        : [{ issue: row.issue.id, version: row.version }],
    ),
    action: statusAction(status),
  };
}

/** Changes the status of the issues of a preview, each on the version the preview read. */
export async function changeMany(
  session: ApiSession,
  preview: BulkPreview,
  status: NewStatus,
): Promise<BulkResult> {
  return dataOf(
    await session.client.POST("/api/v1/bulk/changes", { body: bulkBody(preview, status) }),
  );
}

/** The status an issue has now. */
export async function statusOf(session: ApiSession, issue: MadeIssue): Promise<string | undefined> {
  return (await readIssue(session, issue)).issue.status ?? undefined;
}

/** The changes of an issue before the scenario, to tell afterwards which events are new. */
export interface Known {
  readonly issue: MadeIssue;
  readonly before: Changes;
}

/** Remembers the changes of each issue as they are now. */
export function knowing(session: ApiSession, issues: readonly MadeIssue[]): Promise<Known[]> {
  return Promise.all(
    issues.map(async (issue) => ({ issue, before: await changesOf(session, issue) })),
  );
}

/** Each issue got `fields` as the fields of one new event, and no other. */
export async function expectOneEventEach(
  session: ApiSession,
  known: readonly Known[],
  fields: readonly string[],
): Promise<void> {
  const fresh = await Promise.all(
    known.map(({ issue, before }) => changesSince(session, issue, before)),
  );
  expect(fresh).toEqual(known.map(() => [fields]));
}

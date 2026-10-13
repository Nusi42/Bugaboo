import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { type MadeIssue } from "./issues.ts";

/** The changes in the history of an issue: for each event, the fields it changed. */
export type Changes = Readonly<Record<string, readonly string[]>>;

/** The changes in the history of an issue by their event. */
export async function changesOf(session: ApiSession, issue: MadeIssue): Promise<Changes> {
  const history = dataOf(
    await session.client.GET("/api/v1/issues/{issue}/history", {
      params: { path: { issue: issue.key }, query: { tab: "changes", limit: 100 } },
    }),
  );
  const events: Record<string, string[]> = {};
  for (const item of history.items) {
    if (item.kind === "change") {
      events[item.event] = [...(events[item.event] ?? []), item.field ?? ""];
    }
  }
  return events;
}

/** The fields of each event that came after those of `before`. */
export async function changesSince(
  session: ApiSession,
  issue: MadeIssue,
  before: Changes,
): Promise<readonly (readonly string[])[]> {
  const now = await changesOf(session, issue);
  const fresh: (readonly string[])[] = [];
  for (const [event, fields] of Object.entries(now)) {
    if (!(event in before)) {
      fresh.push(fields);
    }
  }
  return fresh;
}

/** The issue as the session reads it. */
export async function readIssue(session: ApiSession, issue: MadeIssue) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: issue.key } } }),
  );
}

/** How many own fields have a value in the issue. */
export async function ownFieldsSet(session: ApiSession, issue: MadeIssue): Promise<number> {
  return Object.keys((await readIssue(session, issue)).issue.own_fields ?? {}).length;
}

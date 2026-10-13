import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { type MadeIssue } from "./issues.ts";

/** The changes in the history of an issue: for each event, the fields it changed. */
export type Changes = Readonly<Record<string, readonly string[]>>;

/** One page of the history of an issue, and where the next page starts. */
async function historyPage(
  session: ApiSession,
  issue: MadeIssue,
  tab: "changes" | "comments",
  cursor?: string,
) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}/history", {
      params: {
        path: { issue: issue.key },
        query: { tab, limit: 100, ...(cursor === undefined ? {} : { cursor }) },
      },
    }),
  );
}

/** The changes in the history of an issue by their event, every page of it. */
export async function changesOf(
  session: ApiSession,
  issue: MadeIssue,
  cursor?: string,
  found: Readonly<Record<string, readonly string[]>> = {},
): Promise<Changes> {
  const page = await historyPage(session, issue, "changes", cursor);
  const events: Record<string, readonly string[]> = { ...found };
  for (const item of page.items) {
    if (item.kind === "change") {
      events[item.event] = [...(events[item.event] ?? []), item.field ?? ""];
    }
  }
  return page.next_cursor === null || page.next_cursor === undefined
    ? events
    : changesOf(session, issue, page.next_cursor, events);
}

/** How many comments the history of an issue holds, every page of it. */
export async function commentsOf(
  session: ApiSession,
  issue: MadeIssue,
  cursor?: string,
  counted = 0,
): Promise<number> {
  const page = await historyPage(session, issue, "comments", cursor);
  const total = counted + page.items.length;
  return page.next_cursor === null || page.next_cursor === undefined
    ? total
    : commentsOf(session, issue, page.next_cursor, total);
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

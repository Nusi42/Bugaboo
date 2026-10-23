import { dataOf, pageSizeMax, type Schema } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";
import { type MadeIssue, readIssue } from "./issues.ts";

/** The changes in the history of an issue: for each event, the fields it changed. */
export type Changes = Readonly<Record<string, readonly string[]>>;

/** One page of the history of an issue, and where the next page starts. */
async function historyPage(
  session: ApiSession,
  issue: MadeIssue,
  tab: Schema<"HistoryTab">,
  cursor?: string,
) {
  return dataOf(
    await session.client.GET("/api/v1/issues/{issue}/history", {
      params: {
        path: { issue: issue.key },
        query: { tab, limit: pageSizeMax, ...(cursor === undefined ? {} : { cursor }) },
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
      events[item.event] = [...(events[item.event] ?? []), item.change?.field ?? ""];
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

/** A field or a choice by its ID and the name a person reads. */
interface Named {
  readonly id: string;
  readonly name: string;
}

/** A field with its choices. */
interface Known extends Named {
  readonly choices: readonly Named[];
}

/**
 * The choice that each pick-one own field of the issue has, by the names of the field and of the
 * choice (as a person reads them). A value that is no choice of its field is shown as it is, so a
 * wrong value is named in the failure.
 */
export async function ownChoices(
  session: ApiSession,
  issue: MadeIssue,
): Promise<Readonly<Record<string, string>>> {
  const values = (await readIssue(session, issue)).issue.own_fields ?? {};
  const fields: Known[] = [];
  for (const field of dataOf(await session.client.GET("/api/v1/fields")).fields) {
    const choices: Named[] = [];
    for (const choice of field.choices) {
      choices.push({ id: choice.id, name: choice.name });
    }
    fields.push({ id: field.id, name: field.name, choices });
  }
  const choices: Record<string, string> = {};
  for (const [id, value] of Object.entries(values)) {
    const field = fields.find((each) => each.id === id);
    const choice = field?.choices.find((each) => each.id === value);
    choices[field?.name ?? id] = choice?.name ?? JSON.stringify(value);
  }
  return choices;
}

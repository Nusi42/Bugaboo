import { dataOf } from "@bugaboo/api-client";

import { ada, signInThroughApi, type Person } from "./accounts.ts";
import { issueOf } from "./issues.ts";
import type { Instance } from "./server.ts";
import { changeIssue, ownDates } from "./timeline.ts";

// What the tests of the timeline read and write through the API, as a person would elsewhere: the
// dates of an issue, its own fields and its relations.

/** The internal IDs of issues, as the administrator reads them. */
export async function idsOf(server: Instance, keys: readonly string[]): Promise<readonly string[]> {
  const admin = await signInThroughApi(server, ada);
  const ids = await Promise.all(keys.map(async (key) => (await issueOf(admin, key)).id));
  await admin.dispose();
  return ids;
}

/** The own date fields *Begins* and *Ends*, made by the administrator: their IDs. */
export async function createOwnDates(
  server: Instance,
): Promise<{ readonly begins: string; readonly ends: string }> {
  const admin = await signInThroughApi(server, ada);
  const create = async (name: string): Promise<string> => {
    const made = dataOf(
      await admin.client.POST("/api/v1/fields", {
        body: { name, kind: "date", applies_to: "issues" },
      }),
    );
    return made.id ?? "";
  };
  const begins = await create(ownDates.begins);
  const ends = await create(ownDates.ends);
  await admin.dispose();
  return { begins, ends };
}

/** Gives an issue the values of these own fields, as the administrator. */
export async function setOwnValues(
  server: Instance,
  key: string,
  fields: Readonly<Record<string, string>>,
): Promise<void> {
  const admin = await signInThroughApi(server, ada);
  await changeIssue(admin, key, { fields });
  await admin.dispose();
}

/** The value an issue has in an own field, as the administrator reads it. */
export async function ownValueOf(server: Instance, key: string, field: string): Promise<unknown> {
  const admin = await signInThroughApi(server, ada);
  const read = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await admin.dispose();
  return read.issue.own_fields?.[field];
}

/** The issues related to an issue, as text, as the administrator reads them. */
export async function relationsOf(server: Instance, key: string): Promise<string> {
  const admin = await signInThroughApi(server, ada);
  const relations = dataOf(
    await admin.client.GET("/api/v1/issues/{issue}/related-issues", {
      params: { path: { issue: key } },
    }),
  );
  await admin.dispose();
  return JSON.stringify(relations);
}

/** The dates an issue keeps, as a person reads them. */
export async function datesAs(server: Instance, person: Person, key: string) {
  const session = await signInThroughApi(server, person);
  const read = dataOf(
    await session.client.GET("/api/v1/issues/{issue}", { params: { path: { issue: key } } }),
  );
  await session.dispose();
  return { start: read.issue.start_date ?? null, due: read.issue.due_date ?? null };
}

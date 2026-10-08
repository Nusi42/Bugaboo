import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";

/** The ID of the row of *Who has which role* for the whole organization that is for this person, if any. */
export async function rowOfPerson(
  session: ApiSession,
  person: string,
): Promise<string | undefined> {
  const { items } = dataOf(await session.client.GET("/api/v1/assignments"));
  return (
    items.find((row) => row.person?.shown === "name" && row.person.id === person)?.id ?? undefined
  );
}

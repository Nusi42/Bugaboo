import { type ApiSession } from "./api.ts";

/** The ID of the row of *Who has which role* for the whole organization that is for this person, if any. */
export async function rowOfPerson(
  session: ApiSession,
  person: string,
): Promise<string | undefined> {
  const items: unknown = (await session.get("/api/v1/assignments"))["items"];
  const rows: readonly unknown[] = Array.isArray(items) ? items : [];
  const found = rows.find((row) => {
    const who: unknown =
      typeof row === "object" && row !== null ? Reflect.get(row, "person") : null;
    return typeof who === "object" && who !== null && Reflect.get(who, "id") === person;
  });
  const id: unknown =
    typeof found === "object" && found !== null ? Reflect.get(found, "id") : undefined;
  return typeof id === "string" ? id : undefined;
}

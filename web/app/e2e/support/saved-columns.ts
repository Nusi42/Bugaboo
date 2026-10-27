import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";

/** The search words of the columns that the saved search of this name keeps, in order. */
export async function savedColumns(admin: ApiSession, name: string): Promise<readonly string[]> {
  const saved = dataOf(
    await admin.client.GET("/api/v1/saved-searches", { params: { query: { scope: "mine" } } }),
  );
  let id: string | undefined;
  for (const item of saved.items) {
    if (item.name === name) {
      id = item.id;
      break;
    }
  }
  if (id === undefined) {
    return [];
  }
  const one = dataOf(
    await admin.client.GET("/api/v1/saved-searches/{search}", {
      params: { path: { search: id } },
    }),
  );
  return one.presentation.columns.map((each) => each.field);
}

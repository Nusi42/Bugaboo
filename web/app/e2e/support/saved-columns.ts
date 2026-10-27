import { dataOf } from "@bugaboo/api-client";

import { type ApiSession } from "./api.ts";

/** The search words of the columns that the saved search of this name keeps, in order. */
export async function savedColumns(admin: ApiSession, name: string): Promise<readonly string[]> {
  const saved = dataOf(
    await admin.client.GET("/api/v1/saved-searches", { params: { query: { scope: "mine" } } }),
  );
  for (const item of saved.items) {
    if (item.name === name) {
      return item.presentation.columns.map((each) => each.field);
    }
  }
  return [];
}

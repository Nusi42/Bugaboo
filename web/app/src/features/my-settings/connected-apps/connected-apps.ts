import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { fieldList, pageSize } from "../../../shared/paged.ts";

/** An app the person allowed, as the list shows it. */
export type ConnectedApp = Schema<"ConnectedAppItem">;

/** The fields the list asks for: the fields it shows, and no more (PERF-02). */
const LISTED_FIELDS = fieldList<ConnectedApp>([
  "id",
  "name",
  "on_this_computer",
  "scope",
  "projects",
  "allowed_at",
  "last_used_at",
]);

/** The apps the person allowed, newest first. */
export function appsOptions() {
  return api.query.queryOptions("get", "/api/v1/oauth/apps", {
    params: { query: { fields: LISTED_FIELDS, limit: pageSize.largest } },
  });
}

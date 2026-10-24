import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { fieldList, pageSize } from "../../../shared/paged.ts";
import { queryClient } from "../../../shared/query-client.ts";

/** An access key as the list shows it. */
export type AccessKey = Schema<"AccessKeyItem">;

/** The fields the list asks for: the fields it shows, and no more (PERF-02). */
const LISTED_FIELDS = fieldList<AccessKey>([
  "id",
  "name",
  "prefix",
  "choice",
  "permissions",
  "projects",
  "expires_at",
  "last_used_at",
  "expired",
]);

/** The access keys of the person, newest first. */
export function keysOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys", {
    params: { query: { fields: LISTED_FIELDS, limit: pageSize.largest } },
  });
}

/** How long a new key lives in this organization, and the permissions a key can be limited to. */
export function rulesOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys/rules");
}

/** The projects the person can limit a key to. */
export function projectsOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys/project-choices", {
    params: { query: { limit: pageSize.largest } },
  });
}

/** A key was made: the list reads again. */
export function refreshKeys(): void {
  void queryClient.invalidateQueries({ queryKey: keysOptions().queryKey });
}

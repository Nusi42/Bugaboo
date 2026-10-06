import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { queryClient } from "../../../shared/query-client.ts";

/** An access key as the list shows it. */
export type AccessKey = Schema<"AccessKeyItem">;

/** The fields the list asks for: the fields it shows, and no more (PERF-02). */
const LISTED_FIELDS = [
  "id",
  "name",
  "prefix",
  "choice",
  "permissions",
  "projects",
  "expires_at",
  "last_used_at",
  "expired",
].join(",");

/** The most keys a person can have is also the most a page holds, so one page is the whole list. */
const WHOLE_LIST = 100;

/** The access keys of the person, newest first. */
export function keysOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys", {
    params: { query: { fields: LISTED_FIELDS, limit: WHOLE_LIST } },
  });
}

/** How long a new key lives in this organization, and the permissions a key can be limited to. */
export function rulesOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys/rules");
}

/** The projects the person can limit a key to. */
export function projectsOptions() {
  return api.query.queryOptions("get", "/api/v1/access-keys/projects", {
    params: { query: { limit: WHOLE_LIST } },
  });
}

/** A key was made: the list reads again. */
export function refreshKeys(): void {
  void queryClient.invalidateQueries({ queryKey: keysOptions().queryKey });
}

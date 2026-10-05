import { api } from "../../shared/api.ts";

/** The places where the person is signed in. */
export function sessionsOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/sessions");
}

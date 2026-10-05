import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** The places where the person is signed in. */
export function sessionsOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/sessions");
}

/** A session ended: the list is read again. */
export function refreshSessions(): void {
  void queryClient.invalidateQueries({ queryKey: sessionsOptions().queryKey });
}

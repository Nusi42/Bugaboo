import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** How the second step of the person is set up: code app, backup codes and passkeys. */
export function twoStepOptions() {
  return api.query.queryOptions("get", "/api/v1/auth/two-step");
}

/** The second step changed: what the screens show about it is read again. */
export function refreshTwoStep(): void {
  void queryClient.invalidateQueries({ queryKey: twoStepOptions().queryKey });
}

import { api } from "./api.ts";
import { queryClient } from "./query-client.ts";

/**
 * Something about the pins changed, or about what they point to (the name of a project or of a saved search, a search
 * put into the trash): the pins of the person are read again. Written here, not in the pins feature, because the
 * projects and the saved searches say it too.
 */
export function refreshPins(): void {
  void queryClient.invalidateQueries({
    queryKey: api.query.queryOptions("get", "/api/v1/me/pins").queryKey,
  });
}

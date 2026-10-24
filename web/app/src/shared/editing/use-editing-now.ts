import { readEditLock, type EditTarget } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../api.ts";
import { queryKeys } from "../query-keys.ts";

/** Who is editing a thing now, as the person may see them; nothing when nobody is or while it is not asked. */
export function useEditingNow(what: EditTarget, asked: boolean) {
  return useQuery({
    queryKey: queryKeys.editing.now(what),
    queryFn: () => readEditLock(api.client, what),
    enabled: asked,
  });
}

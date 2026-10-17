import { readEditLock } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { whatOf } from "./workflows.ts";

/** Who is editing the workflow now, as the person may see them; nothing when nobody is or while it is not asked. */
export function useEditingNow(workflow: string, asked: boolean) {
  return useQuery({
    queryKey: queryKeys.workflows.editing(workflow),
    queryFn: () => readEditLock(api.client, whatOf(workflow)),
    enabled: asked,
  });
}

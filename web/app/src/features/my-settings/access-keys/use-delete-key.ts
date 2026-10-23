import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../../shared/use-optimistic.ts";
import { keysOptions } from "./access-keys.ts";

type Keys = Schema<"Page_AccessKeyItem">;

/** Whether the cache holds the list of keys. */
function isKeys(value: unknown): value is Keys {
  return typeof value === "object" && value !== null && Array.isArray(Reflect.get(value, "items"));
}

/**
 * Deleting a key. It leaves the list at once and the server is asked in the background (UX-06);
 * when the server refuses, the key comes back with a plain message.
 */
export function useDeleteKey(): OptimisticMutation<{ readonly id: string }> {
  return useOptimisticMutation<{ readonly id: string }, Keys, unknown>({
    queryKey: keysOptions().queryKey,
    isCached: isKeys,
    apply: (before, { id }) =>
      before === undefined
        ? undefined
        : { ...before, items: before.items.filter((one) => one.id !== id) },
    send: ({ id }) =>
      api.client.DELETE("/api/v1/access-keys/{access_key}", {
        params: { path: { access_key: id } },
      }),
  });
}

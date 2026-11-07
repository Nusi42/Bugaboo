import type { Schema } from "@bugaboo/api-client";

import { api } from "../../../shared/api.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../../shared/use-optimistic.ts";
import { appsOptions } from "./connected-apps.ts";

type Apps = Schema<"Page_ConnectedAppItem">;

/** Whether the cache holds the list of apps. */
function isApps(value: unknown): value is Apps {
  return typeof value === "object" && value !== null && Array.isArray(Reflect.get(value, "items"));
}

/**
 * *Take back*: the app leaves the list at once and the server is asked in the background (UX-06);
 * when the server refuses, the app comes back with a plain message.
 */
export function useTakeBack(): OptimisticMutation<{ readonly id: string }> {
  return useOptimisticMutation<{ readonly id: string }, Apps, unknown>({
    queryKey: appsOptions().queryKey,
    isCached: isApps,
    apply: (before, { id }) =>
      before === undefined
        ? undefined
        : { ...before, items: before.items.filter((one) => one.id !== id) },
    send: ({ id }) =>
      api.client.DELETE("/api/v1/oauth/apps/{app}", {
        params: { path: { app: id } },
      }),
  });
}

import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import { sessionsOptions } from "./sessions.ts";

/** Whether the cache holds the list of places. */
function isSessions(value: unknown): value is Schema<"SessionsBody"> {
  return (
    typeof value === "object" && value !== null && Array.isArray(Reflect.get(value, "sessions"))
  );
}

/**
 * Signing out of one other place. The place leaves the list at once and the server is asked in the
 * background (UX-06); when the server refuses, the place comes back with a plain message.
 */
export function useEndSession(): OptimisticMutation<{ readonly id: string }> {
  return useOptimisticMutation<{ readonly id: string }, Schema<"SessionsBody">, unknown>({
    queryKey: sessionsOptions().queryKey,
    isCached: isSessions,
    apply: (before, { id }) =>
      before === undefined
        ? undefined
        : { ...before, sessions: before.sessions.filter((one) => one.id !== id) },
    send: ({ id }) =>
      api.client.DELETE("/api/v1/auth/sessions/{session_id}", {
        params: { path: { session_id: id } },
      }),
  });
}

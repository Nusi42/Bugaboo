import { dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";

import { queryKeys } from "../../shared/query-keys.ts";

/** Asks the server which address an invitation is for, which also shows whether it still works. */
export function useInvitation(secret: string) {
  return useQuery({
    queryKey: queryKeys.invitations.one(secret),
    queryFn: async () =>
      dataOf(await api.client.POST("/api/v1/auth/invitations/lookup", { body: { secret } })),
    // The secret is not kept in memory longer than the page needs it.
    gcTime: 0,
  });
}

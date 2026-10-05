import { ApiError, type Schema } from "@bugaboo/api-client";
import { useInfiniteQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** How many open invitations one page holds. */
const PAGE_SIZE = 20;

/** The fields the list shows: the address, and when the invitation was made and runs out. */
const SHOWN_FIELDS = "id,email,created_at,expires_at";

/** The key of everything about invitations in the query cache. */
const invitationsKey = ["invitations"] as const;

/** The open invitations, a page at a time: *Show more* asks for the next page. */
export function useOpenInvitations() {
  return useInfiniteQuery({
    queryKey: invitationsKey,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/auth/invitations", {
        params: {
          query: {
            fields: SHOWN_FIELDS,
            limit: PAGE_SIZE,
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_InvitationItem">) => last.next_cursor ?? null,
  });
}

/** An invitation was made or taken back: the list is read again. */
export function refreshInvitations(): void {
  void queryClient.invalidateQueries({ queryKey: invitationsKey });
}

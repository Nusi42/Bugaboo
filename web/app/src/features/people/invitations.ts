import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { fieldList, type FieldNames, pageSize, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The fields the list shows: the address, and when the invitation was made and runs out. */
const SHOWN_FIELDS = fieldList(["id", "email", "created_at", "expires_at"] satisfies FieldNames<
  Schema<"InvitationItem">
>);

/** The open invitations, a page at a time: *Show more* asks for the next page. */
export function useOpenInvitations() {
  return usePaged({
    queryKey: queryKeys.invitations.all,
    read: (after) =>
      api.client.GET("/api/v1/auth/invitations", {
        params: { query: { fields: SHOWN_FIELDS, limit: pageSize.short, ...after } },
      }),
  });
}

/** An invitation was made or taken back: the list is read again. */
export function refreshInvitations(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.invitations.all });
}

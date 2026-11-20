import { useState } from "react";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import type { DataKind } from "./keep-rules.ts";

/** The count of the records of a kind past its keep period, read again every time the page opens. */
export function oldRecordsOptions(kind: DataKind) {
  return api.query.queryOptions(
    "get",
    "/api/v1/admin/keep-rules/{kind}/past",
    { params: { path: { kind } } },
    { staleTime: 0 },
  );
}

/**
 * *Delete them*: the records of a kind past its keep period, of everybody or of one person; says how many were
 * deleted. `before` is the date of the count that was shown: the server deletes only while the keep period still gives
 * it, and refuses with `keeping.past-changed` otherwise (the count is then read again). The one way any kind an
 * administrator deletes is deleted (ADR-0045, decision 6).
 */
export function useDeleteOld(kind: DataKind, before: string) {
  const [deleted, setDeleted] = useState<number>();
  const remove = api.query.useMutation("post", "/api/v1/admin/keep-rules/{kind}/delete-past", {
    onSuccess: ({ deleted: count }) => {
      setDeleted(count);
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: oldRecordsOptions(kind).queryKey });
    },
  });
  const send = (person?: string): void => {
    setDeleted(undefined);
    remove.mutate({
      params: {
        path: { kind },
        query: person === undefined ? { before } : { person, before },
      },
    });
  };
  return {
    deleted,
    send,
    error: remove.isError ? remove.error : undefined,
    busy: remove.isPending,
  };
}

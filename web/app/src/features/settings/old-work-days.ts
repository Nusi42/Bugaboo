import { useState } from "react";

import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";

/** The count of the old work days, read again every time the page opens. */
export function oldDaysOptions() {
  return api.query.queryOptions("get", "/api/v1/work-days/old", undefined, { staleTime: 0 });
}

/** Deleting old work days, for everybody or for one person; says how many were deleted. */
export function useDeleteOld() {
  const [deleted, setDeleted] = useState<number>();
  const remove = api.query.useMutation("delete", "/api/v1/work-days/old", {
    onSuccess: ({ deleted: count }) => {
      setDeleted(count);
    },
    onSettled: () => {
      void queryClient.invalidateQueries({ queryKey: oldDaysOptions().queryKey });
    },
  });
  const send = (person?: string): void => {
    setDeleted(undefined);
    remove.mutate({ params: { query: person === undefined ? {} : { person } } });
  };
  return {
    deleted,
    send,
    error: remove.isError ? remove.error : undefined,
    busy: remove.isPending,
  };
}

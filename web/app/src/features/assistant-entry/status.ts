import type { Schema } from "@bugaboo/api-client";
import { keepPreviousData, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";

/** Whether the assistant is there at a place, and where its answers come from. */
export type EntryStatus = Schema<"AssistantStatusBody">;

/**
 * Whether the assistant is there at the place, and where its answers come from. The place is a
 * project, or none for a page of the organization. While the answer for a new place loads, the state of
 * the place before it stays shown; before the first answer, and when the server refuses (a project the
 * person may not see, or one kept from AI), nothing of the assistant is shown.
 */
export function useAssistantStatus(project: string | undefined): EntryStatus | undefined {
  const query = useQuery({
    ...api.query.queryOptions("get", "/api/v1/assistant", {
      params: { query: project === undefined ? {} : { project } },
    }),
    retry: false,
    // The new place is asked while the one before is still shown, so the state of a place that is not known yet is never taken for off.
    placeholderData: keepPreviousData,
  });
  return query.data;
}

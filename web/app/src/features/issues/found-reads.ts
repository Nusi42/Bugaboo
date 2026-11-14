import { ApiError } from "@bugaboo/api-client";
import { keepPreviousData } from "@tanstack/react-query";
import { useState } from "react";

import { api } from "../../shared/api.ts";
import { doNothing } from "../../shared/do-nothing.ts";
import { pageSize, pagedOptions, usePaged } from "../../shared/paged.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { FALLBACK_FIELDS, fieldsFor, queryOf, type FoundSearch } from "./found-issues.ts";

// How the issues a search finds are read: a page at a time for a list, and a first page ahead of
// time, so a list can show its rows in the frame it shows its groups (UX §6.4).

/** Which search the server refused the fields of the columns for: its next pages ask for the fallback at once. */
interface FieldsMemory {
  readonly refused: () => string | null;
  readonly refuse: (searchKey: string) => void;
}

/** A memory of refused fields for one read ahead of time, which keeps nothing after it. */
function freshMemory(): FieldsMemory {
  let refused: string | null = null;
  return {
    refused: () => refused,
    refuse: (searchKey) => {
      refused = searchKey;
    },
  };
}

/** The request for the issues a search finds, with what the columns show. */
export function foundRequest(
  search: FoundSearch,
  columns: readonly string[],
  memory: FieldsMemory,
) {
  const fields = fieldsFor(columns);
  const searchKey = `${search.text}|${search.within ?? ""}|${fields}`;
  const page = (asked: string) => (after: { readonly cursor?: string }) =>
    api.client.GET("/api/v1/search/issues", {
      params: {
        query: { ...queryOf(search), fields: asked, limit: pageSize.usual, ...after },
      },
    });
  return {
    queryKey: queryKeys.search.found(search.text, search.within, fields),
    read: async (after: { readonly cursor?: string }) => {
      if (memory.refused() === searchKey) {
        return page(FALLBACK_FIELDS)(after);
      }
      try {
        return await page(fields)(after);
      } catch (error) {
        if (error instanceof ApiError && error.problem?.code === "request.unknown-field") {
          memory.refuse(searchKey);
          return page(FALLBACK_FIELDS)(after);
        }
        throw error;
      }
    },
  };
}

/** The issues a search finds, a page at a time, with what the columns show. */
export function useFoundIssues(search: FoundSearch, columns: readonly string[], enabled = true) {
  const [memory] = useState(freshMemory);
  return usePaged(foundRequest(search, columns, memory), {
    placeholderData: keepPreviousData,
    enabled,
  });
}

/** Reads the first page of what a search finds ahead of time, so a list can show it at once. */
export async function readFirstFound(
  search: FoundSearch,
  columns: readonly string[],
): Promise<void> {
  // A page that cannot be read is told by its group, which asks again when it is shown.
  await queryClient
    .infiniteQuery(pagedOptions(foundRequest(search, columns, freshMemory())))
    .catch(doNothing);
}

/** Whether the first page of what a search finds is read already. */
export function hasFirstFound(search: FoundSearch, columns: readonly string[]): boolean {
  return (
    queryClient.getQueryData(foundRequest(search, columns, freshMemory()).queryKey) !== undefined
  );
}

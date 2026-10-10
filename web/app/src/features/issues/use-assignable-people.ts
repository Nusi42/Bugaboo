import type { Schema } from "@bugaboo/api-client";
import { keepPreviousData } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, type NamedPerson } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** The people an issue of the project can be assigned to whose names contain the words, a page at a time. */
export function useAssignablePeople(project: string, words: string) {
  return usePaged(
    {
      queryKey: queryKeys.issues.people(project, words),
      read: (after) =>
        api.client.GET("/api/v1/projects/{project}/assignable-people", {
          params: {
            path: { project },
            query: { ...(words === "" ? {} : { search: words }), limit: pageSize.short, ...after },
          },
        }),
    },
    { placeholderData: keepPreviousData },
  );
}

/** The people in the pages read so far that are shown by name: the only ones that can be chosen. */
export function peopleOf(pages: readonly Schema<"Page_PersonRef">[] | undefined): NamedPerson[] {
  return linesFrom(pages, namedPerson);
}

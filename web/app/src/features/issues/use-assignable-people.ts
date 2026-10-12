import type { Schema } from "@bugaboo/api-client";
import { keepPreviousData } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, type NamedPerson } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** What the list of people is for besides *Assigned to*: an own field of people, and the issue it is for. */
export interface PeopleFor {
  /** The ID of the own field: the list then needs the right to change that field. */
  readonly field?: string | undefined;
  /** The ID of the issue: who reported it decides who may see it. */
  readonly issue?: string | undefined;
}

/** The people an issue of the project can be assigned to whose names contain the words, a page at a time. */
export function useAssignablePeople(project: string, words: string, wanted: PeopleFor = {}) {
  const { field, issue } = wanted;
  return usePaged(
    {
      queryKey: queryKeys.issues.people(project, words, field, issue),
      read: (after) =>
        api.client.GET("/api/v1/projects/{project}/assignable-people", {
          params: {
            path: { project },
            query: {
              ...(words === "" ? {} : { search: words }),
              ...(field === undefined ? {} : { field }),
              ...(issue === undefined ? {} : { issue }),
              limit: pageSize.short,
              ...after,
            },
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

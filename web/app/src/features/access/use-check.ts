import { type Schema, dataOf } from "@bugaboo/api-client";
import { keepPreviousData, skipToken, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { namedPerson, type NamedPerson } from "../../shared/person.ts";

/** What *Check access* answers for a person in a place. */
export type CheckAnswerBody = Schema<"CheckAccessBody">;

/** The people the person who checks can choose from, found by words in the name, a page at a time. */
export function usePeopleToCheck(project: string | undefined, search: string) {
  return usePaged(
    {
      queryKey: queryKeys.checkAccess.people(project, search),
      read: (after) =>
        api.client.GET("/api/v1/check-access/people", {
          params: {
            query: {
              ...(project === undefined ? {} : { project }),
              ...(search === "" ? {} : { search }),
              limit: pageSize.short,
              ...after,
            },
          },
        }),
    },
    { placeholderData: keepPreviousData },
  );
}

/** A project to choose as the place to look at. */
export interface PlaceChoice {
  readonly id: string;
  readonly shortName: string;
  readonly name: string;
}

/** The projects the person sees that the words name, a page at a time (an empty text lists them all). */
export function useProjectsToChoose(search: string) {
  return usePaged(
    {
      queryKey: queryKeys.checkAccess.projects(search),
      read: (after) =>
        api.client.GET("/api/v1/projects", {
          params: {
            query: {
              search,
              fields: fieldList<Schema<"ProjectItem">>(["id", "short_name", "name"]),
              limit: pageSize.short,
              ...after,
            },
          },
        }),
    },
    { placeholderData: keepPreviousData },
  );
}

/** The places of the pages read so far: those that came with everything a line needs. */
export function placesOf(
  pages: readonly Schema<"Page_ProjectItem">[] | undefined,
): readonly PlaceChoice[] {
  return linesFrom(pages, (item) => {
    const { id, short_name: shortName, name } = item;
    return id === undefined || id === null || shortName === undefined || shortName === null
      ? undefined
      : { id, shortName, name: name ?? shortName };
  });
}

/** The people of the pages read so far. Only a person shown by name can be chosen: nothing else has an ID. */
export function peopleOf(
  pages: readonly Schema<"Page_PersonRef">[] | undefined,
): readonly NamedPerson[] {
  return linesFrom(pages, namedPerson);
}

/**
 * What a person can do in a place and why: asked once a person is chosen. With an issue type (an
 * ID), the field levels are those for issues of that type.
 */
export function useCheckAccess(
  person: string | undefined,
  project: string | undefined,
  issueType: string | undefined,
) {
  return useQuery({
    queryKey: queryKeys.checkAccess.answer(person, project, issueType),
    // A check is a question about now: it is never kept for later.
    gcTime: 0,
    queryFn:
      person === undefined
        ? skipToken
        : async () =>
            dataOf(
              await api.client.GET("/api/v1/check-access/people/{person}", {
                params: {
                  path: { person },
                  query: {
                    ...(project === undefined ? {} : { project }),
                    ...(issueType === undefined ? {} : { issue_type: issueType }),
                  },
                },
              }),
            ),
  });
}

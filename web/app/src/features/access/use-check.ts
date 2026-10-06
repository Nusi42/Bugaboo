import { ApiError, type Schema } from "@bugaboo/api-client";
import { keepPreviousData, useInfiniteQuery, useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";

/** What *Check access* answers for a person in a place. */
export type CheckAnswerBody = Schema<"CheckAccessBody">;
/** One person to choose from, as the person who asks may see them: by name, with the ID. */
export type PersonChoice = Schema<"PersonRef">;

/** How many people or projects one page of a list to choose from holds. */
const PAGE_SIZE = 20;

/** The people the person who checks can choose from, found by words in the name, a page at a time. */
export function usePeopleToCheck(project: string | undefined, search: string) {
  return useInfiniteQuery({
    queryKey: ["check-access", "people", project ?? null, search],
    placeholderData: keepPreviousData,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/check-access/people", {
        params: {
          query: {
            ...(project === undefined ? {} : { project }),
            ...(search === "" ? {} : { search }),
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
    getNextPageParam: (last: Schema<"Page_PersonRef">) => last.next_cursor ?? null,
  });
}

/** A project to choose as the place to look at. */
export interface PlaceChoice {
  readonly id: string;
  readonly shortName: string;
  readonly name: string;
}

/** The projects the person sees that the words name, a page at a time (an empty text lists them all). */
export function useProjectsToChoose(search: string) {
  return useInfiniteQuery({
    queryKey: ["check-access", "projects", search],
    placeholderData: keepPreviousData,
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/projects", {
        params: {
          query: {
            search,
            fields: "id,short_name,name",
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
    getNextPageParam: (last: Schema<"Page_ProjectItem">) => last.next_cursor ?? null,
  });
}

/** The places of the pages read so far: those that came with everything a line needs. */
export function placesOf(
  pages: readonly Schema<"Page_ProjectItem">[] | undefined,
): readonly PlaceChoice[] {
  return (pages ?? []).flatMap((page) =>
    page.items.flatMap((item) => {
      const { id, short_name: shortName, name } = item;
      return id === undefined || id === null || shortName === undefined || shortName === null
        ? []
        : [{ id, shortName, name: name ?? shortName }];
    }),
  );
}

/** A person to choose from: the ID and the name. */
export interface PersonLine {
  readonly id: string;
  readonly name: string;
}

/** The people of the pages read so far. Only a person shown by name can be chosen: nothing else has an ID. */
export function peopleOf(
  pages: readonly Schema<"Page_PersonRef">[] | undefined,
): readonly PersonLine[] {
  return (pages ?? []).flatMap((page) =>
    page.items.flatMap((item) => (item.shown === "name" ? [{ id: item.id, name: item.name }] : [])),
  );
}

/** What a person can do in a place and why: asked once a person is chosen. */
export function useCheckAccess(person: string | undefined, project: string | undefined) {
  return useQuery({
    queryKey: ["check-access", "answer", person ?? null, project ?? null],
    enabled: person !== undefined,
    // A check is a question about now: it is never kept for later.
    gcTime: 0,
    queryFn: async () => {
      if (person === undefined) {
        throw new ApiError(0, undefined);
      }
      const { data } = await api.client.GET("/api/v1/check-access/people/{person}", {
        params: { path: { person }, query: project === undefined ? {} : { project } },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

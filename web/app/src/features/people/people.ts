import { ApiError, type Schema } from "@bugaboo/api-client";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

import { peopleKey } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";

/** A person in a list, as the server sends it: the fields can be missing. */
export type ListItem = Schema<"PersonItem">;

/** How many people one page holds. */
const PAGE_SIZE = 50;

/** The fields the list and the page of a person need. */
const SHOWN_FIELDS = "person,email,groups,roles";

/** A person that has what a line of the list needs (the fields can be missing in an answer). */
export interface PersonLine {
  readonly id: string;
  readonly name: string;
  readonly email: string;
  /** The groups the person is in, when the viewer may see groups. */
  readonly groups: readonly Schema<"GroupRef">[] | null;
  readonly roles: readonly Schema<"RoleAtRef">[];
}

/** The line of a person, or nothing for one that is not shown by name (nobody who may ask is). */
export function lineOf(item: ListItem): PersonLine | undefined {
  const { person } = item;
  if (person === null || person === undefined || person.shown !== "name") {
    return undefined;
  }
  return {
    id: person.id,
    name: person.name,
    email: item.email ?? "",
    groups: item.groups ?? null,
    roles: item.roles ?? [],
  };
}

/** The lines of the pages read so far. */
export function linesOf(pages: readonly Schema<"Page_PersonItem">[] | undefined): PersonLine[] {
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => lineOf(item) ?? []));
}

/** The people that the search finds, a page at a time (all of them for an empty search). */
export function usePeople(search: string) {
  return useInfiniteQuery({
    queryKey: [...peopleKey, "list", search],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/people", {
        params: {
          query: {
            fields: SHOWN_FIELDS,
            limit: PAGE_SIZE,
            ...(search === "" ? {} : { search }),
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_PersonItem">) => last.next_cursor ?? null,
  });
}

/** One person with their groups and roles. */
export function usePerson(id: string) {
  return useQuery({
    queryKey: [...peopleKey, "one", id],
    queryFn: async () => {
      const { data } = await api.client.GET("/api/v1/people/{person_id}", {
        params: { path: { person_id: id } },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

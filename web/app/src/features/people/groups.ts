import { ApiError, type Schema } from "@bugaboo/api-client";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

import { groupsKey } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";

/** How many groups one page holds. */
const PAGE_SIZE = 100;
/** How many people a page of members or choices holds. */
const MEMBERS_SIZE = 50;
const CHOICES_SIZE = 20;

/** A group that has what a line of a list needs (the fields can be missing in an answer). */
export interface GroupLine {
  readonly id: string;
  readonly name: string;
}

/** The line of a group, or nothing for an item that came without what it needs. */
export function groupLineOf(item: Schema<"GroupItem">): GroupLine | undefined {
  const { id, name } = item;
  return id === undefined || id === null || name === undefined || name === null
    ? undefined
    : { id, name };
}

/** The groups in the pages read so far. */
export function groupLinesOf(pages: readonly Schema<"Page_GroupItem">[] | undefined): GroupLine[] {
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => groupLineOf(item) ?? []));
}

/** A person of a group, or a person to put into one: shown by name. */
export interface PersonLine {
  readonly id: string;
  readonly name: string;
}

/** The groups, a page at a time. */
export function useGroups() {
  return useInfiniteQuery({
    queryKey: [...groupsKey, "list"],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/groups", {
        params: {
          query: {
            fields: "id,name",
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
    getNextPageParam: (last: Schema<"Page_GroupItem">) => last.next_cursor ?? null,
  });
}

/** One group with what it is used for. */
export function useGroup(id: string) {
  return useQuery({
    queryKey: [...groupsKey, "one", id],
    queryFn: async () => {
      const { data } = await api.client.GET("/api/v1/groups/{group_id}", {
        params: { path: { group_id: id } },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

/** The people in a group, a page at a time. */
export function useMembers(group: string) {
  return useInfiniteQuery({
    queryKey: [...groupsKey, "members", group],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/groups/{group_id}/members", {
        params: {
          path: { group_id: group },
          query: {
            fields: "person",
            limit: MEMBERS_SIZE,
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_MemberItem">) => last.next_cursor ?? null,
  });
}

/** The people that can be put into a group, found by name, a page at a time. */
export function useMemberChoices(group: string, search: string) {
  return useInfiniteQuery({
    queryKey: [...groupsKey, "choices", group, search],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/groups/{group_id}/member-choices", {
        params: {
          path: { group_id: group },
          query: {
            fields: "id,name,already_in",
            limit: CHOICES_SIZE,
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
    getNextPageParam: (last: Schema<"Page_ChoiceItem">) => last.next_cursor ?? null,
  });
}

/** A person found by the search for people to put into a group. */
export interface Choice {
  readonly id: string;
  readonly name: string;
  /** Whether the person is in the group already. */
  readonly alreadyIn: boolean;
}

/** The people in the pages of choices read so far. */
export function choicesOf(pages: readonly Schema<"Page_ChoiceItem">[] | undefined): Choice[] {
  return (pages ?? []).flatMap((page) =>
    page.items.flatMap((item) =>
      item.id === undefined || item.id === null || item.name === undefined || item.name === null
        ? []
        : [{ id: item.id, name: item.name, alreadyIn: item.already_in === true }],
    ),
  );
}

/** The people in the pages of members read so far, shown by name. */
export function membersOf(pages: readonly Schema<"Page_MemberItem">[] | undefined): PersonLine[] {
  return (pages ?? []).flatMap((page) =>
    page.items.flatMap((item) => {
      const person = item.person;
      return person?.shown === "name" ? [{ id: person.id, name: person.name }] : [];
    }),
  );
}

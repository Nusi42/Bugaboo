import { type Schema, dataOf } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import { fieldList, linesFrom, pageSize, usePaged } from "../../shared/paged.ts";
import { namedPerson, type NamedPerson } from "../../shared/person.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A person of a group, or a person to put into one: shown by name. */
export type Member = NamedPerson;

/** A group that has what a line of a list needs (the fields can be missing in an answer). */
export interface GroupLine {
  readonly id: string;
  readonly name: string;
  /** Counts up with every change; a change is made on the version that was looked at. */
  readonly version: number;
}

/** The line of a group, or nothing for an item that came without what it needs. */
export function groupLineOf(item: Schema<"GroupItem">): GroupLine | undefined {
  const { id, name, version } = item;
  return id === undefined ||
    id === null ||
    name === undefined ||
    name === null ||
    version === undefined ||
    version === null
    ? undefined
    : { id, name, version };
}

/** The groups in the pages read so far. */
export function groupLinesOf(pages: readonly Schema<"Page_GroupItem">[] | undefined): GroupLine[] {
  return linesFrom(pages, groupLineOf);
}

/** The groups, a page at a time. */
export function useGroups() {
  return usePaged({
    queryKey: queryKeys.groups.list(),
    read: (after) =>
      api.client.GET("/api/v1/groups", {
        params: {
          query: {
            fields: fieldList<Schema<"GroupItem">>(["id", "name", "version"]),
            limit: pageSize.largest,
            ...after,
          },
        },
      }),
  });
}

/** One group with what it is used for. */
export function useGroup(id: string) {
  return useQuery({
    queryKey: queryKeys.groups.one(id),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/groups/{group}", {
          params: { path: { group: id } },
        }),
      ),
  });
}

/** The people in a group, a page at a time. */
export function useMembers(group: string) {
  return usePaged({
    queryKey: queryKeys.groups.members(group),
    read: (after) =>
      api.client.GET("/api/v1/groups/{group}/members", {
        params: {
          path: { group: group },
          query: {
            fields: fieldList<Schema<"MemberItem">>(["person"]),
            limit: pageSize.usual,
            ...after,
          },
        },
      }),
  });
}

/** The people that can be put into a group, found by name, a page at a time. */
export function useMemberChoices(group: string, search: string) {
  return usePaged({
    queryKey: queryKeys.groups.choices(group, search),
    read: (after) =>
      api.client.GET("/api/v1/groups/{group}/member-choices", {
        params: {
          path: { group: group },
          query: {
            fields: fieldList<Schema<"ChoiceItem">>(["id", "name", "already_in"]),
            limit: pageSize.short,
            ...(search === "" ? {} : { search }),
            ...after,
          },
        },
      }),
  });
}

/** A person found by the search for people to put into a group. */
export interface Choice extends NamedPerson {
  /** Whether the person is in the group already. */
  readonly alreadyIn: boolean;
}

/** The people in the pages of choices read so far. */
export function choicesOf(pages: readonly Schema<"Page_ChoiceItem">[] | undefined): Choice[] {
  return linesFrom(pages, (item) =>
    item.id === undefined || item.id === null || item.name === undefined || item.name === null
      ? undefined
      : { id: item.id, name: item.name, alreadyIn: item.already_in === true },
  );
}

/** The people in the pages of members read so far, shown by name. */
export function membersOf(pages: readonly Schema<"Page_MemberItem">[] | undefined): NamedPerson[] {
  return linesFrom(pages, (item) => namedPerson(item.person));
}

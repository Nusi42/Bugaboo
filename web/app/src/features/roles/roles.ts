import type { Schema } from "@bugaboo/api-client";
import { useQuery } from "@tanstack/react-query";

import { api } from "../../shared/api.ts";
import {
  dataOf,
  fieldList,
  type FieldNames,
  linesFrom,
  pageSize,
  usePaged,
} from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";

/** A role that has what a line of a list needs (the fields can be missing in an answer). */
export interface RoleLine {
  readonly id: string;
  readonly name: string;
  /** The starting role it is a copy of, if it is one. */
  readonly starting: string | null;
  readonly twoStep: boolean;
}

/** The line of a role, or nothing for an item that came without what it needs. */
export function roleLineOf(item: Schema<"RoleItem">): RoleLine | undefined {
  const { id, name } = item;
  if (id === undefined || id === null || name === undefined || name === null) {
    return undefined;
  }
  return { id, name, starting: item.starting ?? null, twoStep: item.two_step === true };
}

/** The roles in the pages read so far. */
export function roleLinesOf(pages: readonly Schema<"Page_RoleItem">[] | undefined): RoleLine[] {
  return linesFrom(pages, roleLineOf);
}

/** Which roles a list holds: all of them, or only those the person may give. */
export type RolesWanted = "all" | "giveable";

/**
 * The roles, in the order of roles, a page at a time. With `giveable`, only the roles the person
 * may give: they hold every permission of them.
 */
export function useRoles(wanted: RolesWanted = "all") {
  return usePaged({
    queryKey: queryKeys.roles.list(wanted),
    read: (after) =>
      api.client.GET("/api/v1/roles", {
        params: {
          query: {
            fields: fieldList(["id", "name", "starting", "two_step"] satisfies FieldNames<
              Schema<"RoleItem">
            >),
            limit: pageSize.largest,
            ...(wanted === "giveable" ? { giveable: true } : {}),
            ...after,
          },
        },
      }),
  });
}

/** One role with its permissions and what it is used for. */
export function useRole(id: string) {
  return useQuery({
    queryKey: queryKeys.roles.one(id),
    queryFn: async () =>
      dataOf(
        await api.client.GET("/api/v1/roles/{role_id}", {
          params: { path: { role_id: id } },
        }),
      ),
  });
}

/** The permissions of the catalogue, topic by topic. */
export function useCatalogue() {
  return useQuery({
    queryKey: queryKeys.roles.catalogue(),
    staleTime: Infinity,
    queryFn: async () => dataOf(await api.client.GET("/api/v1/permissions")),
  });
}

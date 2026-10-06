import { ApiError, type Schema } from "@bugaboo/api-client";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";

import { rolesKey } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";

/** How many roles one page holds. */
const PAGE_SIZE = 100;

/** One permission of the catalogue, by its key. */
export type PermissionKey = Schema<"Permission">;

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
  return (pages ?? []).flatMap((page) => page.items.flatMap((item) => roleLineOf(item) ?? []));
}

/**
 * The roles, in the order of roles, a page at a time. With `giveable`, only the roles the person
 * may give: they hold every permission of them.
 */
export function useRoles(giveable = false) {
  return useInfiniteQuery({
    queryKey: [...rolesKey, "list", giveable],
    initialPageParam: null as string | null,
    queryFn: async ({ pageParam }: { readonly pageParam: string | null }) => {
      const { data } = await api.client.GET("/api/v1/roles", {
        params: {
          query: {
            fields: "id,name,starting,two_step",
            limit: PAGE_SIZE,
            ...(giveable ? { giveable } : {}),
            ...(pageParam === null ? {} : { cursor: pageParam }),
          },
        },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
    getNextPageParam: (last: Schema<"Page_RoleItem">) => last.next_cursor ?? null,
  });
}

/** The key of one role in the query cache. */
export function roleKey(id: string): readonly string[] {
  return [...rolesKey, "one", id];
}

/** One role with its permissions and what it is used for. */
export function useRole(id: string) {
  return useQuery({
    queryKey: roleKey(id),
    queryFn: async () => {
      const { data } = await api.client.GET("/api/v1/roles/{role_id}", {
        params: { path: { role_id: id } },
      });
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

/** The permissions of the catalogue, topic by topic. */
export function useCatalogue() {
  return useQuery({
    queryKey: [...rolesKey, "catalogue"],
    staleTime: Infinity,
    queryFn: async () => {
      const { data } = await api.client.GET("/api/v1/permissions");
      if (data === undefined) {
        throw new ApiError(0, undefined);
      }
      return data;
    },
  });
}

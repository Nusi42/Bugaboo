import type { Schema } from "@bugaboo/api-client";
import { useMemo } from "react";

import { api } from "../../shared/api.ts";
import { fieldList, itemsOf, pageSize, useAllPages, usePaged } from "../../shared/paged.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { rowsOf, useAssignments, type Subject } from "../roles/index.ts";
import { whoSeesWorkingHours } from "./who-sees.ts";

/** The roles with what they allow, a page at a time. */
function useRolesWithPermissions() {
  return usePaged({
    queryKey: queryKeys.roles.list("with-permissions"),
    read: (after) =>
      api.client.GET("/api/v1/roles", {
        params: {
          query: {
            fields: fieldList<Schema<"RoleItem">>(["id", "name", "permissions"]),
            limit: pageSize.largest,
            ...after,
          },
        },
      }),
  });
}

/**
 * The people and groups that hold the permission today, read through the routes of roles and of
 * *Who has which role* (no route of its own, so there is one place that says who holds what).
 */
export function useWhoSeesWorkingHours(): {
  readonly holders: readonly Subject[] | undefined;
  readonly failed: unknown;
} {
  const roles = useRolesWithPermissions();
  const assignments = useAssignments(null);
  useAllPages(roles);
  useAllPages(assignments);
  const loading =
    roles.isPending ||
    assignments.isPending ||
    roles.hasNextPage ||
    assignments.hasNextPage ||
    roles.isFetchingNextPage ||
    assignments.isFetchingNextPage;
  const failed = roles.error ?? assignments.error;
  const holders = useMemo(
    () =>
      loading || failed !== null
        ? undefined
        : whoSeesWorkingHours(itemsOf(roles.data?.pages), rowsOf(assignments.data?.pages)),
    [loading, failed, roles.data, assignments.data],
  );
  return { holders, failed };
}

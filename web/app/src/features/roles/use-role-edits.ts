import type { Schema } from "@bugaboo/api-client";

import { api } from "../../shared/api.ts";
import { inOrder } from "../../shared/in-order.ts";
import { queryClient } from "../../shared/query-client.ts";
import { queryKeys } from "../../shared/query-keys.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import type { PermissionKey } from "../../shared/permissions.ts";

type Detail = Schema<"RoleDetailBody">;

/** What a person changes on the page of a role: the permissions as a whole, or the switch. */
export type RoleEdit =
  | {
      readonly kind: "permissions";
      readonly permissions: readonly PermissionKey[];
      /** The version of the role on screen: the one the change is made on. */
      readonly version: number;
    }
  | { readonly kind: "two-step"; readonly on: boolean; readonly version: number };

/** Whether the cache holds the page of a role. */
function isDetail(value: unknown): value is Detail {
  return typeof value === "object" && value !== null && "role" in value && "used_by" in value;
}

/** The page of the role as it looks after an edit, before the server has answered. */
export function after(before: Detail | undefined, edit: RoleEdit): Detail | undefined {
  if (before === undefined) {
    return undefined;
  }
  const changed =
    edit.kind === "permissions"
      ? { ...before.role, permissions: [...edit.permissions] }
      : { ...before.role, two_step: edit.on };
  // Contract with `update_role` on the server: every change counts the version up with
  // `bb_core::version::next` (exactly one), so the next change on screen is made on that one.
  return { ...before, role: { ...changed, version: edit.version + 1 } };
}

/** Sends an edit to the server and tells what shows the role elsewhere to read it again. */
async function send(id: string, edit: RoleEdit): Promise<void> {
  const body =
    edit.kind === "permissions"
      ? { version: edit.version, permissions: [...edit.permissions] }
      : { version: edit.version, two_step: edit.on };
  // One after the other: each change is made on the version the one before it has just made.
  await inOrder(`role:${id}`, () =>
    api.client.PATCH("/api/v1/roles/{role_id}", { params: { path: { role_id: id } }, body }),
  );
  // The list of roles and what people and rows say of the role; the page itself is read again by
  // the helper once the last edit is done, so overlapping edits do not make it flicker.
  void queryClient.invalidateQueries({ queryKey: queryKeys.roles.lists() });
  void queryClient.invalidateQueries({ queryKey: queryKeys.people.all });
  void queryClient.invalidateQueries({ queryKey: queryKeys.assignments.all });
}

/** The permissions and the two-step switch of one role, changed at once (UX-06): *Saving…*, *Saved*. */
export function useRoleEdits(id: string): OptimisticMutation<RoleEdit> {
  return useOptimisticMutation<RoleEdit, Detail, void>({
    queryKey: queryKeys.roles.one(id),
    isCached: isDetail,
    apply: after,
    send: (edit) => send(id, edit),
  });
}

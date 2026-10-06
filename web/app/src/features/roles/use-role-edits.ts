import type { Schema } from "@bugaboo/api-client";

import { assignmentsKey, peopleKey, rolesKey } from "../../shared/access-cache.ts";
import { api } from "../../shared/api.ts";
import { queryClient } from "../../shared/query-client.ts";
import { useOptimisticMutation, type OptimisticMutation } from "../../shared/use-optimistic.ts";
import { type PermissionKey, roleKey } from "./roles.ts";

type Detail = Schema<"RoleDetailBody">;

/** What a person changes on the page of a role: the permissions as a whole, or the switch. */
export type RoleEdit =
  | { readonly kind: "permissions"; readonly permissions: readonly PermissionKey[] }
  | { readonly kind: "two-step"; readonly on: boolean };

/** Whether the cache holds the page of a role. */
function isDetail(value: unknown): value is Detail {
  return typeof value === "object" && value !== null && "role" in value && "used_by" in value;
}

/** The page of the role as it looks after an edit, before the server has answered. */
export function after(before: Detail | undefined, edit: RoleEdit): Detail | undefined {
  if (before === undefined) {
    return undefined;
  }
  const role =
    edit.kind === "permissions"
      ? { ...before.role, permissions: [...edit.permissions] }
      : { ...before.role, two_step: edit.on };
  return { ...before, role };
}

/** Sends an edit to the server and tells what shows the role elsewhere to read it again. */
async function send(id: string, edit: RoleEdit): Promise<void> {
  const body =
    edit.kind === "permissions" ? { permissions: [...edit.permissions] } : { two_step: edit.on };
  await api.client.PATCH("/api/v1/roles/{role_id}", { params: { path: { role_id: id } }, body });
  // The list of roles and what people and rows say of the role; the page itself is read again by
  // the helper once the last edit is done, so overlapping edits do not make it flicker.
  void queryClient.invalidateQueries({ queryKey: [...rolesKey, "list"] });
  void queryClient.invalidateQueries({ queryKey: peopleKey });
  void queryClient.invalidateQueries({ queryKey: assignmentsKey });
}

/** The permissions and the two-step switch of one role, changed at once (UX-06): *Saving…*, *Saved*. */
export function useRoleEdits(id: string): OptimisticMutation<RoleEdit> {
  return useOptimisticMutation<RoleEdit, Detail, void>({
    queryKey: roleKey(id),
    isCached: isDetail,
    apply: after,
    send: (edit) => send(id, edit),
  });
}

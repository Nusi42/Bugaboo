import type { Schema } from "@bugaboo/api-client";

import type { PermissionKey } from "../../shared/permissions.ts";
import type { Subject } from "../roles/index.ts";

/** The permission that shows working hours of other people (ADR-0035, decision 8). */
const SEE_OTHERS: PermissionKey = "see-working-hours-of-others";

/** The roles that hold the permission to see the working hours of others. */
function rolesThatSee(roles: readonly Schema<"RoleItem">[]): ReadonlySet<string> {
  return new Set(
    roles.flatMap((role) =>
      role.id !== undefined &&
      role.id !== null &&
      (role.permissions ?? []).some((permission) => permission === SEE_OTHERS)
        ? [role.id]
        : [],
    ),
  );
}

/** Who holds the permission, for the whole organization: each person or group once, in the order of the rows. */
export function whoSeesWorkingHours(
  roles: readonly Schema<"RoleItem">[],
  rows: readonly { readonly who: Subject; readonly role: { readonly id: string } | null }[],
): readonly Subject[] {
  const wanted = rolesThatSee(roles);
  const seen = new Set<string>();
  return rows.flatMap(({ who, role }) => {
    const key = `${who.kind}:${who.id}`;
    if (role === null || !wanted.has(role.id) || seen.has(key)) {
      return [];
    }
    seen.add(key);
    return [who];
  });
}

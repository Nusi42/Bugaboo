import type { Schema } from "@bugaboo/api-client";

/** One permission of the catalogue, as the API names it (`manage-invitations`). */
export type PermissionKey = Schema<"Permission">;

/**
 * What a person may do for the whole organization: the answer of `/auth/me`. The menus follow it;
 * the server decides again at every request.
 */
export type Permissions = readonly PermissionKey[];

/** Nothing: what a person who is not known yet may do. */
export const noPermissions: Permissions = [];

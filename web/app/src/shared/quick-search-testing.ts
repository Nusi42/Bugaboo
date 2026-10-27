import { doNothing } from "./do-nothing.ts";
import type { Permissions } from "./permissions.ts";

/** Somebody who asks quick search in a test: no permissions, and signing out does nothing. */
export const asker: { readonly permissions: Permissions; readonly signOut: () => void } = {
  permissions: [],
  signOut: doNothing,
};

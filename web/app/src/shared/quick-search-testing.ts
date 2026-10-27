import { doNothing } from "./do-nothing.ts";
import { noPermissions } from "./permissions.ts";

/**
 * Somebody who asks quick search in a test: no permissions, and a way to sign out that does
 * nothing, or the one the test watches. Its type is whatever quick search asks for, checked where
 * it is used, so the shape of an asker is written once, in the shell.
 */
export function askerFor(signOut: () => void = doNothing) {
  return { permissions: noPermissions, signOut };
}

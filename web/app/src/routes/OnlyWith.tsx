import type { ReactNode } from "react";

import { mayOpen, NotFoundPage } from "../features/shell/index.ts";
import { usePermissions } from "../shared/use-me.ts";

/**
 * A screen that asks for a permission: a person who does not hold one is shown "not found", as if
 * the screen did not exist (M2-F17). The list of screens (`places.ts`) says which permission each
 * one asks for, so there is no second list.
 */
export function OnlyWith({
  path,
  children,
}: {
  readonly path: string;
  readonly children: ReactNode;
}) {
  return mayOpen(path, usePermissions()) ? children : <NotFoundPage />;
}

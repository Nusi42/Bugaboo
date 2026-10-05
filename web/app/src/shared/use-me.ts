import { useQuery } from "@tanstack/react-query";

import { noPermissions, type Permissions } from "./permissions.ts";
import { meOptions, type Person } from "./session.ts";

/** Who is signed in, once the app knows (it asked at the start). */
export function useMe(): Person | undefined {
  return useQuery(meOptions()).data;
}

/** What the person who is signed in may do for the whole organization. */
export function usePermissions(): Permissions {
  return useMe()?.permissions ?? noPermissions;
}

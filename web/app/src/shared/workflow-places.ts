import { iconName } from "@bugaboo/ui/icons";

import type { Place } from "./place.ts";
import { paths } from "./paths.ts";

/** *Organization settings › Workflows*: the statuses and status changes of the whole organization. */
export const workflowPlaces: readonly Place[] = [
  {
    path: paths.workflows,
    name: "workflows.nav",
    icon: iconName.status,
    parent: paths.organizationSettings,
    anyOf: ["manage-workflows"],
    group: "nav.group.work",
    listed: true,
  },
];

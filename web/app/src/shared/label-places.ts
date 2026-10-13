import { iconName } from "@bugaboo/ui/icons";

import type { Place } from "./place.ts";
import { paths } from "./paths.ts";

/** *Organization settings › Labels and tags*. */
export const labelPlaces: readonly Place[] = [
  {
    path: paths.labels,
    name: "nav.labels",
    icon: iconName.labels,
    parent: paths.organizationSettings,
    anyOf: ["manage-labels"],
    group: "nav.group.work",
    listed: true,
  },
];

import { iconName } from "@bugaboo/ui/icons";

import type { Place } from "./place.ts";
import { paths } from "./paths.ts";

/** *Organization settings › Kinds of relation*. */
export const relationPlaces: readonly Place[] = [
  {
    path: paths.relationKinds,
    name: "nav.relation-kinds",
    icon: iconName["relation-kinds"],
    parent: paths.organizationSettings,
    anyOf: ["manage-issue-types"],
    group: "nav.group.work",
    listed: true,
  },
];

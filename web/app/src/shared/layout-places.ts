import { iconName } from "@bugaboo/ui/icons";

import type { Place } from "./place.ts";
import { paths } from "./paths.ts";

/** *Organization settings › Layouts*: the forms of the whole organization. */
export const layoutPlaces: readonly Place[] = [
  {
    path: paths.layouts,
    name: "layouts.nav",
    icon: iconName.panel,
    parent: paths.organizationSettings,
    anyOf: ["manage-issue-types"],
    group: "nav.group.work",
    listed: true,
  },
];

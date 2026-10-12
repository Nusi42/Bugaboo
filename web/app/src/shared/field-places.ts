import { iconName } from "@bugaboo/ui/icons";

import type { Place } from "./place.ts";
import { paths } from "./paths.ts";

/** *Organization settings › Fields* and the page of one field. */
export const fieldPlaces: readonly Place[] = [
  {
    path: paths.fields,
    name: "nav.fields",
    icon: iconName.fields,
    parent: paths.organizationSettings,
    anyOf: ["manage-fields"],
    group: "nav.group.work",
    listed: true,
  },
  {
    // A field's page is reached from the list of fields.
    path: paths.field,
    name: "nav.fields",
    icon: iconName.fields,
    parent: paths.fields,
    anyOf: ["manage-fields"],
    listed: false,
  },
];

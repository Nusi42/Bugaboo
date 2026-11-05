import { iconName } from "@bugaboo/ui/icons";

import { layoutPlaces } from "./layout-places.ts";
import { paths } from "./paths.ts";
import type { Place } from "./place.ts";
import { workflowPlaces } from "./workflow-places.ts";

/** *Organization settings › Work › Working hours*: the rules, the keep period and who sees what. */
const workingHoursPlace: Place = {
  path: paths.workingHours,
  name: "nav.working-hours",
  icon: iconName.clock,
  parent: paths.organizationSettings,
  anyOf: ["change-organization-settings"],
  group: "nav.group.work",
  listed: true,
};

/** The screens of *Organization settings › Work* that sit next to each other: layouts, workflows and working hours. */
export const workPlaces: readonly Place[] = [...layoutPlaces, ...workflowPlaces, workingHoursPlace];

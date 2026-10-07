import { iconName } from "@bugaboo/ui";

import { paths } from "../../shared/paths.ts";
import type { Place } from "./place.ts";

/** The pages of a project: they name themselves (`usePageTrail`), so they are here only to be known. */
export const projectPlaces: readonly Place[] = [
  {
    // The address of a project has a part to fill in: the screen is here so that it is known,
    // the pages of the projects name themselves (`usePageTrail`).
    path: paths.project,
    name: "nav.projects",
    icon: iconName.projects,
    parent: paths.home,
    listed: false,
  },
  {
    path: paths.projectSettings,
    name: "projects.settings-link",
    icon: iconName.settings,
    parent: paths.project,
    listed: false,
  },
  {
    // Who has which role in a project: the pages of the projects name themselves
    // (`usePageTrail`), the screen is here so that it is known.
    path: paths.projectRoles,
    name: "roles.project-link",
    icon: iconName.roles,
    parent: paths.projectSettings,
    listed: false,
  },
  {
    path: paths.projectFields,
    name: "nav.field-permissions",
    icon: iconName["field-permissions"],
    parent: paths.projectSettings,
    listed: false,
  },
  {
    path: paths.projectPeople,
    name: "nav.people-visibility",
    icon: iconName["people-visibility"],
    parent: paths.projectSettings,
    listed: false,
  },
  {
    path: paths.projectCheckAccess,
    name: "nav.check-access",
    icon: iconName["check-access"],
    parent: paths.projectSettings,
    listed: false,
  },
];

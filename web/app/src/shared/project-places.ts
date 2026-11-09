import { projectPages, settingsTabs } from "./project-pages.ts";
import type { Place } from "./place.ts";

/** The pages of a project: they name themselves (`usePageTrail`), so they are here only to be known. */
export const projectPlaces: readonly Place[] = [
  ...[projectPages.overview, projectPages.issues].map(({ path, name, icon, parent }) => ({
    path,
    name,
    icon,
    parent,
    listed: false,
  })),
  // The settings of a project are in the area of settings, like those of the organization.
  ...settingsTabs.map(({ path, name, icon, parent }) => ({
    path,
    name,
    icon,
    parent,
    area: "settings" as const,
    listed: false,
  })),
];

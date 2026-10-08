import { projectPages, settingsTabs } from "../../shared/project-pages.ts";
import type { Place } from "./place.ts";

/** The pages of a project: they name themselves (`usePageTrail`), so they are here only to be known. */
export const projectPlaces: readonly Place[] = [projectPages.overview, ...settingsTabs].map(
  ({ path, name, icon, parent }) => ({ path, name, icon, parent, listed: false }),
);

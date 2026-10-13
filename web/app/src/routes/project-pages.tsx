import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

const ProjectOverviewScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectOverviewPage",
);
const ProjectSettingsScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectSettingsPage",
);

/** The overview of a project, `/p/WEB`. The project decides who sees it: nobody else finds it. */
export const projectRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.project,
  loader: loadsScreen(ProjectOverviewScreen),
  component: function Project() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectOverviewScreen address={project} />;
  },
});

/** The settings of a project, `/p/WEB/settings`. */
export const projectSettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectSettings,
  loader: loadsScreen(ProjectSettingsScreen),
  component: function ProjectSettings() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectSettingsScreen address={project} />;
  },
});

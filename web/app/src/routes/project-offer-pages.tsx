import { createRoute, lazyRouteComponent } from "@tanstack/react-router";

import { paths, routeParam } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute } from "./layouts.tsx";
import { loadsScreen } from "./preload.ts";

// The settings of a project that say what it offers (issue types and groups of labels), loaded when
// they are first opened (coding-standards section 6).

const ProjectIssueTypesScreen = lazyRouteComponent(
  () => import("../features/issue-types/index.ts"),
  "ProjectIssueTypesScreen",
);

/** `/p/WEB/settings/issue-types`: the issue types of a project, and which of them it offers. */
export const projectIssueTypesRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectIssueTypes,
  loader: loadsScreen(ProjectIssueTypesScreen),
  component: function ProjectIssueTypes() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectIssueTypesScreen address={project} />;
  },
});

const ProjectLabelsScreen = lazyRouteComponent(
  () => import("../features/labels/index.ts"),
  "ProjectLabelsScreen",
);

/** `/p/WEB/settings/labels`: the groups of labels of a project, and which of them it offers. */
export const projectLabelsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectLabels,
  loader: loadsScreen(ProjectLabelsScreen),
  component: function ProjectLabels() {
    const project = useAddressPart(routeParam.project);
    return project === undefined ? null : <ProjectLabelsScreen address={project} />;
  },
});

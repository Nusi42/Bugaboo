import type { Schema } from "@bugaboo/api-client";
import { createRoute, lazyRouteComponent, useParams } from "@tanstack/react-router";

import { paths } from "../shared/paths.ts";
import { appRoute } from "./layouts.tsx";

const ProjectOverviewScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectOverviewPage",
);
const ProjectSettingsScreen = lazyRouteComponent(
  () => import("../features/projects/index.ts"),
  "ProjectSettingsPage",
);
const ProjectAccessCardScreen = lazyRouteComponent(
  () => import("../features/access/index.ts"),
  "ProjectAccessCardScreen",
);

/** The short name (or ID) in the address of a project page, if the address has one. */
export function useProjectOfAddress(): string | undefined {
  const params: unknown = useParams({ strict: false });
  const project: unknown =
    typeof params === "object" && params !== null ? Reflect.get(params, "project") : undefined;
  return typeof project === "string" ? project : undefined;
}

/** The card that leads to what roles see in the project, after the cards of the project settings. */
function accessCard(detail: Schema<"ProjectDetailBody">) {
  return <ProjectAccessCardScreen project={detail} />;
}

/** The overview of a project, `/p/WEB`. The project decides who sees it: nobody else finds it. */
export const projectRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.project,
  component: function Project() {
    const project = useProjectOfAddress();
    return project === undefined ? null : <ProjectOverviewScreen address={project} />;
  },
});

/**
 * The settings of a project, `/p/WEB/settings`, with the card of what roles see here. The two
 * features do not know each other: the route puts them together.
 */
export const projectSettingsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectSettings,
  component: function ProjectSettings() {
    const project = useProjectOfAddress();
    return project === undefined ? null : (
      <ProjectSettingsScreen address={project} extra={accessCard} />
    );
  },
});

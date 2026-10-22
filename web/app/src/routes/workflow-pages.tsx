import { lazyRouteComponent } from "@tanstack/react-router";

import { paths, type WorkflowsOpen, workflowsAddress } from "../shared/paths.ts";
import { placeEditorRoutes } from "./place-editor-routes.tsx";

/** What the address of the editor of workflows says is open: the issue type. */
function workflowsSearch(search: Readonly<Record<string, unknown>>): WorkflowsOpen {
  return { type: typeof search["type"] === "string" ? search["type"] : undefined };
}

const routes = placeEditorRoutes({
  organizationPath: paths.workflows,
  projectPath: paths.projectWorkflows,
  search: workflowsSearch,
  address: workflowsAddress,
  OrganizationScreen: lazyRouteComponent(
    () => import("../features/workflows/index.ts"),
    "OrganizationWorkflowsScreen",
  ),
  ProjectScreen: lazyRouteComponent(
    () => import("../features/workflows/index.ts"),
    "ProjectWorkflowsScreen",
  ),
});

/** *Organization settings › Workflows*: the statuses and status changes of the whole organization. */
export const workflowsRoute = routes.organization;

/** `/p/WEB/settings/workflows`: the workflows of a project. */
export const projectWorkflowsRoute = routes.project;

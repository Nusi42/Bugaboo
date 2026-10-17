import { createRoute, lazyRouteComponent, useNavigate, useSearch } from "@tanstack/react-router";

import { paths, routeParam, type WorkflowsOpen, workflowsAddress } from "../shared/paths.ts";
import { useAddressPart } from "./address-part.ts";
import { appRoute, organizationSettingsRoute } from "./layouts.tsx";
import { OnlyWith } from "./OnlyWith.tsx";
import { loadsScreen } from "./preload.ts";

const OrganizationWorkflowsScreen = lazyRouteComponent(
  () => import("../features/workflows/index.ts"),
  "OrganizationWorkflowsScreen",
);
const ProjectWorkflowsScreen = lazyRouteComponent(
  () => import("../features/workflows/index.ts"),
  "ProjectWorkflowsScreen",
);

/** What the address of the editor of workflows says is open: the issue type. */
function workflowsSearch(search: Readonly<Record<string, unknown>>): WorkflowsOpen {
  return { type: typeof search["type"] === "string" ? search["type"] : undefined };
}

/** *Organization settings › Workflows*: the statuses and status changes of the whole organization. */
export const workflowsRoute = createRoute({
  getParentRoute: () => organizationSettingsRoute,
  path: paths.workflows,
  validateSearch: workflowsSearch,
  loader: loadsScreen(OrganizationWorkflowsScreen),
  component: function Workflows() {
    const open = useSearch({ strict: false });
    const navigate = useNavigate();
    return (
      <OnlyWith path={paths.workflows}>
        <OrganizationWorkflowsScreen
          open={workflowsSearch(open)}
          onOpen={(_, next: WorkflowsOpen) => {
            void navigate({ href: workflowsAddress(undefined, next) });
          }}
        />
      </OnlyWith>
    );
  },
});

/** `/p/WEB/settings/workflows`: the workflows of a project. */
export const projectWorkflowsRoute = createRoute({
  getParentRoute: () => appRoute,
  path: paths.projectWorkflows,
  validateSearch: workflowsSearch,
  loader: loadsScreen(ProjectWorkflowsScreen),
  component: function ProjectWorkflows() {
    const project = useAddressPart(routeParam.project);
    const open = useSearch({ strict: false });
    const navigate = useNavigate();
    return project === undefined ? null : (
      <ProjectWorkflowsScreen
        address={project}
        open={workflowsSearch(open)}
        onOpen={(shortName: string | undefined, next: WorkflowsOpen) => {
          void navigate({ href: workflowsAddress(shortName ?? project, next) });
        }}
      />
    );
  },
});
